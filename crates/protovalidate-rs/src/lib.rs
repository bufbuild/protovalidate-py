//! Safe interface to the C shim over protovalidate-cc.
//!
//! The raw ABI mirrors `shim/pv_shim.h`, which this crate's build script
//! compiles together with the protovalidate-cc sources, and stays private:
//! [`Engine`] is the API.

use std::ffi::{CStr, c_char, c_int};
use std::ptr;

const PV_OK: c_int = 0;
const PV_ERR_COMPILATION: c_int = 1;
const PV_ERR_RUNTIME: c_int = 2;
const PV_ERR_ARGUMENT: c_int = 3;
const PV_ERR_UNEXPECTED: c_int = 4;

/// Opaque raw engine handle.
#[repr(C)]
struct PvEngine {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    fn pv_engine_new(error: *mut *mut c_char) -> *mut PvEngine;
    fn pv_engine_free(engine: *mut PvEngine);
    fn pv_engine_add_file(
        engine: *mut PvEngine,
        file_descriptor_proto: *const u8,
        len: usize,
        error: *mut *mut c_char,
    ) -> c_int;
    fn pv_engine_validate(
        engine: *mut PvEngine,
        type_name: *const c_char,
        type_name_len: usize,
        payload: *const u8,
        payload_len: usize,
        fail_fast: c_int,
        out: *mut *mut u8,
        out_len: *mut usize,
        error: *mut *mut c_char,
    ) -> c_int;
    fn pv_free(ptr: *mut u8);
}

/// An engine failure, classified by the shim's status code.
pub enum PvError {
    Compilation(String),
    Evaluation(String),
    Argument(String),
    Unexpected(String),
    Unknown(i32, String),
}

/// Takes ownership of a shim-allocated error string.
unsafe fn take_error(error: *mut c_char) -> String {
    if error.is_null() {
        return "unknown error".to_owned();
    }
    let message = unsafe { CStr::from_ptr(error) }
        .to_string_lossy()
        .into_owned();
    unsafe { pv_free(error.cast()) };
    message
}

/// Takes ownership of a shim-allocated error string, classified by `code`.
unsafe fn status_error(code: c_int, error: *mut c_char) -> PvError {
    let message = unsafe { take_error(error) };
    match code {
        PV_ERR_COMPILATION => PvError::Compilation(message),
        PV_ERR_RUNTIME => PvError::Evaluation(message),
        PV_ERR_ARGUMENT => PvError::Argument(message),
        PV_ERR_UNEXPECTED => PvError::Unexpected(message),
        code => PvError::Unknown(code, message),
    }
}

/// The C++ engine.
pub struct Engine(*mut PvEngine);

// SAFETY: the engine has no thread affinity, so it may move between threads.
unsafe impl Send for Engine {}
// SAFETY: protovalidate-cc documents `validate` as thread-safe, while `add_file` requires exclusive access.
unsafe impl Sync for Engine {}

impl Engine {
    pub fn new() -> Result<Self, String> {
        let mut error: *mut c_char = ptr::null_mut();
        let engine = unsafe { pv_engine_new(&raw mut error) };
        if engine.is_null() {
            return Err(unsafe { take_error(error) });
        }
        Ok(Self(engine))
    }

    /// Adds a serialized `FileDescriptorProto` to the engine's pool.
    pub fn add_file(&mut self, bytes: &[u8]) -> Result<(), PvError> {
        let mut error: *mut c_char = ptr::null_mut();
        let code =
            unsafe { pv_engine_add_file(self.0, bytes.as_ptr(), bytes.len(), &raw mut error) };
        if code == PV_OK {
            Ok(())
        } else {
            Err(unsafe { status_error(code, error) })
        }
    }

    /// Validates a serialized message against the rules of `type_name`,
    /// returning the serialized `buf.validate.Violations`, or `None` when the
    /// message is valid.
    pub fn validate(
        &self,
        type_name: &str,
        payload: &[u8],
        fail_fast: bool,
    ) -> Result<Option<PvBuffer>, PvError> {
        let mut error: *mut c_char = ptr::null_mut();
        let mut out: *mut u8 = ptr::null_mut();
        let mut out_len: usize = 0;
        let code = unsafe {
            pv_engine_validate(
                self.0,
                // The shim takes pointer + length -- the pool lookup is an
                // absl::string_view -- so the name needs no NUL terminator
                // and no copy.
                type_name.as_ptr().cast::<c_char>(),
                type_name.len(),
                payload.as_ptr(),
                payload.len(),
                c_int::from(fail_fast),
                &raw mut out,
                &raw mut out_len,
                &raw mut error,
            )
        };
        if code != PV_OK {
            return Err(unsafe { status_error(code, error) });
        }
        if out_len == 0 {
            unsafe { pv_free(out) };
            return Ok(None);
        }
        Ok(Some(PvBuffer {
            ptr: out,
            len: out_len,
        }))
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { pv_engine_free(self.0) };
    }
}

/// A byte buffer owned by the shim, freed on drop.
pub struct PvBuffer {
    ptr: *mut u8,
    len: usize,
}

// SAFETY: exclusively owned, never aliased.
unsafe impl Send for PvBuffer {}

impl PvBuffer {
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: the shim wrote `len` initialized bytes, owned until drop.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for PvBuffer {
    fn drop(&mut self) {
        unsafe { pv_free(self.ptr) };
    }
}
