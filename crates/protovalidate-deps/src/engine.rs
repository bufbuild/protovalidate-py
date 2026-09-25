// Copyright (c) 2023-2026 Buf Technologies, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The safe API over the shim: engines, programs, frames and the
//! marshalling of values across the boundary.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
use std::slice;
use std::sync::Arc;

use crate::ffi::{
    CEL_ERR_ARGUMENT, CEL_ERR_COMPILATION, CEL_ERR_RUNTIME, CEL_OK, CEL_THIS_FIELD,
    CEL_THIS_MESSAGE, CEL_THIS_SCALAR, CEL_VALUE_BOOL, CEL_VALUE_BYTES, CEL_VALUE_DOUBLE,
    CEL_VALUE_INT, CEL_VALUE_LIST, CEL_VALUE_STRING, CEL_VALUE_UINT, CelEngine, CelFrame, CelList,
    CelProgram, CelRule, CelValue, cel_engine_add_file, cel_engine_free, cel_engine_new,
    cel_engine_register, cel_frame_free, cel_frame_new, cel_free, cel_list_get, cel_list_len,
    cel_program_eval, cel_program_free, cel_program_new, cel_string_new,
};
use crate::{Arg, Element, Error, Expression, Kind, NativeFn, ScalarValue, This, Value};

/// The shim hands lengths to protobuf as `int`, so a buffer at or above 2 GiB
/// would narrow to a negative one. Protobuf cannot represent a message that
/// large so it is safe to reject.
fn check_len(what: &str, bytes: &[u8]) -> Result<(), Error> {
    if i32::try_from(bytes.len()).is_ok() {
        Ok(())
    } else {
        Err(Error::Argument(format!("{what} is too large")))
    }
}

/// Takes ownership of a shim-allocated error string.
unsafe fn take_error(error: *mut c_char) -> String {
    if error.is_null() {
        return "unknown error".to_owned();
    }
    // SAFETY: the shim allocated a NUL-terminated string the caller has not
    // released, so it stays live until the free below.
    let message = unsafe { CStr::from_ptr(error) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: shim-allocated, and this is the last use of the pointer.
    unsafe { cel_free(error.cast()) };
    message
}

/// The outcome of a shim call that reports a status code, and on any code
/// but `CEL_OK` stores an error string the caller owns in `error`.
unsafe fn finish(code: c_int, error: *mut c_char) -> Result<(), Error> {
    if code == CEL_OK {
        return Ok(());
    }
    // SAFETY: a non-OK code means the shim stored an owned message.
    let message = unsafe { take_error(error) };
    Err(match code {
        CEL_ERR_COMPILATION => Error::Compilation(message),
        CEL_ERR_RUNTIME => Error::Runtime(message),
        CEL_ERR_ARGUMENT => Error::Argument(message),
        code => Error::Unexpected(format!("unknown status {code}: {message}")),
    })
}

impl ScalarValue<'_> {
    fn to_ffi(self) -> CelValue {
        let mut value = CelValue::default();
        match self {
            Self::Bool(b) => {
                value.kind = CEL_VALUE_BOOL;
                value.bool_value = c_int::from(b);
            }
            Self::Int(i) => {
                value.kind = CEL_VALUE_INT;
                value.int_value = i;
            }
            Self::Uint(u) => {
                value.kind = CEL_VALUE_UINT;
                value.uint_value = u;
            }
            Self::Double(d) => {
                value.kind = CEL_VALUE_DOUBLE;
                value.double_value = d;
            }
            Self::String(s) => {
                value.kind = CEL_VALUE_STRING;
                value.data = s.as_ptr();
                value.len = s.len();
            }
            Self::Bytes(b) => {
                value.kind = CEL_VALUE_BYTES;
                value.data = b.as_ptr();
                value.len = b.len();
            }
        }
        value
    }
}

impl Kind {
    fn code(self) -> i32 {
        match self {
            Self::Bool => CEL_VALUE_BOOL,
            Self::Int => CEL_VALUE_INT,
            Self::Uint => CEL_VALUE_UINT,
            Self::Double => CEL_VALUE_DOUBLE,
            Self::String => CEL_VALUE_STRING,
            Self::Bytes => CEL_VALUE_BYTES,
            Self::List => CEL_VALUE_LIST,
        }
    }
}

/// The bytes a `cel_value` points at, borrowed for the call.
unsafe fn ffi_bytes<'a>(value: &CelValue) -> &'a [u8] {
    if value.data.is_null() {
        return &[];
    }
    // SAFETY: the shim points `data` at `len` bytes that outlive the call.
    unsafe { slice::from_raw_parts(value.data, value.len) }
}

/// A list element, read through the shim. `index` is in range.
unsafe fn element<'a>(list: *const CelList, index: usize) -> Element<'a> {
    let mut value = CelValue::default();
    // SAFETY: `list` is the live list of the current call, `index` is below
    // its length, and `value` is a live out-param.
    unsafe { cel_list_get(list, index, &raw mut value) };
    match value.kind {
        CEL_VALUE_BOOL => Element::Bool(value.bool_value != 0),
        CEL_VALUE_INT => Element::Int(value.int_value),
        CEL_VALUE_UINT => Element::Uint(value.uint_value),
        CEL_VALUE_DOUBLE => Element::Double(value.double_value),
        // SAFETY: a string value points at bytes that outlive the call.
        CEL_VALUE_STRING => Element::String(String::from_utf8_lossy(unsafe { ffi_bytes(&value) })),
        // SAFETY: a bytes value points at bytes that outlive the call.
        CEL_VALUE_BYTES => Element::Bytes(unsafe { ffi_bytes(&value) }),
        _ => Element::Other,
    }
}

/// An argument of a registered function, as the shim hands it over.
unsafe fn argument<'a>(value: &CelValue) -> Result<Arg<'a>, String> {
    Ok(match value.kind {
        CEL_VALUE_BOOL => Arg::Bool(value.bool_value != 0),
        CEL_VALUE_INT => Arg::Int(value.int_value),
        CEL_VALUE_UINT => Arg::Uint(value.uint_value),
        CEL_VALUE_DOUBLE => Arg::Double(value.double_value),
        // SAFETY: a string value points at bytes that outlive the call.
        CEL_VALUE_STRING => Arg::String(String::from_utf8_lossy(unsafe { ffi_bytes(value) })),
        // SAFETY: a bytes value points at bytes that outlive the call.
        CEL_VALUE_BYTES => Arg::Bytes(unsafe { ffi_bytes(value) }),
        CEL_VALUE_LIST => {
            // SAFETY: `list` is the live list of the current call.
            let len = unsafe { cel_list_len(value.list) };
            let mut elements = Vec::with_capacity(len);
            for index in 0..len {
                // SAFETY: as above, and `index` is below the length just read.
                elements.push(unsafe { element(value.list, index) });
            }
            Arg::List(elements)
        }
        kind => return Err(format!("unsupported argument kind {kind}")),
    })
}

/// The shim's entry into a registered function: `ctx` is the [`NativeFn`]
/// itself, carried through the shim as an address.
unsafe extern "C" fn call_function(
    ctx: *mut c_void,
    args: *const CelValue,
    len: usize,
    out: *mut c_int,
    error: *mut *mut c_char,
) -> c_int {
    // SAFETY: `ctx` is the address `register` made from a `NativeFn`, which
    // is `'static`, so it is a valid function pointer.
    let function: NativeFn = unsafe { std::mem::transmute(ctx.addr()) };
    // SAFETY: `args` is valid for `len` values for the call.
    let values = unsafe { slice::from_raw_parts(args, len) };
    let result = values
        .iter()
        // SAFETY: each value's data outlives the call.
        .map(|value| unsafe { argument(value) })
        .collect::<Result<Vec<_>, _>>()
        .and_then(|arguments| function(&arguments));
    match result {
        Ok(value) => {
            // SAFETY: `out` is a live out-param.
            unsafe { out.write(c_int::from(value)) };
            CEL_OK
        }
        Err(message) => {
            // SAFETY: `error` is a live out-param, and the shim releases the
            // string it allocates here.
            unsafe {
                error.write(cel_string_new(
                    message.as_ptr().cast::<c_char>(),
                    message.len(),
                ));
            }
            CEL_ERR_RUNTIME
        }
    }
}

/// The shim's engine, freed once the [`Engine`] and every [`Program`] and
/// [`Frame`] made from it are gone: a program's expressions are bound to
/// its function registry, and a frame's message to its message factory,
/// for as long as they live.
struct Shared(*mut CelEngine);

// SAFETY: the engine has no thread affinity, so it may move between threads.
unsafe impl Send for Shared {}
// SAFETY: what programs and frames reach through the engine, the function
// registry and the message factory, is read-only once they exist; the
// calls that write to the engine are on `Engine`, whose `&mut self` the
// borrow checker serializes, and the descriptor pool they add to locks
// internally against the reads.
unsafe impl Sync for Shared {}

impl Drop for Shared {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from `cel_engine_new` and is freed once, after
        // its last program and frame.
        unsafe { cel_engine_free(self.0) };
    }
}

/// The CEL environment: a descriptor pool for the messages expressions see,
/// the functions they can call, and the expression builder.
pub struct Engine {
    shared: Arc<Shared>,
}

impl Engine {
    fn raw(&self) -> *mut CelEngine {
        self.shared.0
    }

    /// Creates an engine with CEL's own functions and the well-known types.
    ///
    /// # Errors
    ///
    /// Fails if the runtime cannot be initialized, which indicates a broken
    /// build rather than bad input.
    pub fn new() -> Result<Self, Error> {
        let mut error: *mut c_char = ptr::null_mut();
        // SAFETY: `error` is a live out-param the shim writes only on failure.
        let raw = unsafe { cel_engine_new(&raw mut error) };
        if raw.is_null() {
            // SAFETY: a null engine means the shim stored an owned message.
            return Err(Error::Unexpected(unsafe { take_error(error) }));
        }
        Ok(Self {
            shared: Arc::new(Shared(raw)),
        })
    }

    /// Makes `function` callable from expressions as `name`, on arguments of
    /// the given kinds; `receiver` makes the first argument the receiver, as
    /// in `this.isEmail()`. The same name may be registered for several kind
    /// lists, which CEL resolves by argument type. Register before compiling.
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] for a registration CEL rejects, such as a
    /// duplicate.
    pub fn register(
        &mut self,
        name: &str,
        receiver: bool,
        args: &[Kind],
        function: NativeFn,
    ) -> Result<(), Error> {
        let kinds: Vec<i32> = args.iter().map(|kind| kind.code()).collect();
        // The shim hands the opaque context back to `call_function`, which
        // is the C-ABI entry point; the function pointer itself travels as
        // that context, so nothing needs to outlive the call but the engine.
        let ctx = ptr::without_provenance_mut::<c_void>(function as usize);
        let mut error: *mut c_char = ptr::null_mut();
        // SAFETY: the engine is live; the name and kinds are valid for their
        // lengths for the call; `error` is a live out-param.
        let code = unsafe {
            cel_engine_register(
                self.raw(),
                name.as_ptr().cast::<c_char>(),
                name.len(),
                c_int::from(receiver),
                kinds.as_ptr(),
                kinds.len(),
                call_function,
                ctx,
                &raw mut error,
            )
        };
        // SAFETY: `code` and `error` are the call's, untouched since.
        unsafe { finish(code, error) }
    }

    /// Adds a serialized `FileDescriptorProto` to the pool. Its imports must
    /// already be there.
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] for a file that does not parse or link.
    pub fn add_file(&mut self, bytes: &[u8]) -> Result<(), Error> {
        check_len("FileDescriptorProto", bytes)?;
        let mut error: *mut c_char = ptr::null_mut();
        // SAFETY: the engine is live, `bytes` is valid for its own length,
        // and `error` is a live out-param.
        let code =
            unsafe { cel_engine_add_file(self.raw(), bytes.as_ptr(), bytes.len(), &raw mut error) };
        // SAFETY: `code` and `error` are the call's, untouched since.
        unsafe { finish(code, error) }
    }

    /// Compiles `expressions` sharing one `rules` message: a serialized
    /// message of the named type, or none.
    ///
    /// # Errors
    ///
    /// [`Error::Compilation`] for an expression that does not compile;
    /// [`Error::Argument`] for a rules message that does not parse.
    pub fn compile(
        &mut self,
        rules: Option<(&str, &[u8])>,
        expressions: &[Expression<'_>],
    ) -> Result<Program, Error> {
        let (type_name, bytes) = rules.unwrap_or(("", &[]));
        check_len("rules message", bytes)?;
        let ffi_rules: Vec<CelRule> = expressions
            .iter()
            .map(|expression| CelRule {
                expression: expression.expression.as_ptr().cast::<c_char>(),
                expression_len: expression.expression.len(),
                rule_field_number: expression.rule_field_number,
            })
            .collect();
        let mut out: *mut CelProgram = ptr::null_mut();
        let mut error: *mut c_char = ptr::null_mut();
        // SAFETY: the engine is live; every pointer/length pair is valid for
        // the duration of the call, which does not retain them; `out` and
        // `error` are live out-params.
        let code = unsafe {
            cel_program_new(
                self.raw(),
                type_name.as_ptr().cast::<c_char>(),
                type_name.len(),
                bytes.as_ptr(),
                bytes.len(),
                ffi_rules.as_ptr(),
                ffi_rules.len(),
                &raw mut out,
                &raw mut error,
            )
        };
        // SAFETY: `code` and `error` are the call's, untouched since.
        unsafe { finish(code, error) }?;
        Ok(Program {
            raw: out,
            _engine: Arc::clone(&self.shared),
        })
    }

    /// Parses `payload` as the named message type.
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] for an unknown type or a payload that does not
    /// parse.
    pub fn frame(&self, type_name: &str, payload: &[u8]) -> Result<Frame, Error> {
        check_len("payload", payload)?;
        let mut out: *mut CelFrame = ptr::null_mut();
        let mut error: *mut c_char = ptr::null_mut();
        // SAFETY: the engine is live; `type_name` and `payload` are valid for
        // their lengths and not retained; `out` and `error` are live
        // out-params.
        let code = unsafe {
            cel_frame_new(
                self.raw(),
                type_name.as_ptr().cast::<c_char>(),
                type_name.len(),
                payload.as_ptr(),
                payload.len(),
                &raw mut out,
                &raw mut error,
            )
        };
        // SAFETY: `code` and `error` are the call's, untouched since.
        unsafe { finish(code, error) }?;
        Ok(Frame {
            raw: out,
            _engine: Arc::clone(&self.shared),
        })
    }
}

/// A set of compiled expressions sharing one `rules` message. It keeps the
/// engine that compiled it alive.
pub struct Program {
    raw: *mut CelProgram,
    _engine: Arc<Shared>,
}

// SAFETY: a program is immutable once built, and evaluation is thread-safe.
unsafe impl Send for Program {}
// SAFETY: as above.
unsafe impl Sync for Program {}

impl Program {
    /// Evaluates expression `index` against `this`, returning what it
    /// produced.
    ///
    /// # Errors
    ///
    /// [`Error::Runtime`] for an expression that fails to evaluate or
    /// produces an error; [`Error::Argument`] for a `this` field that does
    /// not exist.
    pub fn eval(&self, index: usize, this: This<'_>) -> Result<Value, Error> {
        let (kind, scalar, frame, field_number) = match this {
            This::Scalar(scalar) => (CEL_THIS_SCALAR, Some(scalar.to_ffi()), ptr::null(), 0),
            This::Message(frame) => (CEL_THIS_MESSAGE, None, frame.raw.cast_const(), 0),
            This::Field(frame, number) => (CEL_THIS_FIELD, None, frame.raw.cast_const(), number),
        };
        let mut out = CelValue::default();
        let mut error: *mut c_char = ptr::null_mut();
        // SAFETY: `self.raw` is a live program and `index` is one of its
        // expressions; the scalar and its borrowed string storage outlive the
        // call, as does `frame` (`This`'s lifetime); `out` and `error` are
        // live out-params.
        let code = unsafe {
            cel_program_eval(
                self.raw,
                index,
                kind,
                scalar
                    .as_ref()
                    .map_or(ptr::null(), |scalar| &raw const *scalar),
                frame,
                field_number,
                &raw mut out,
                &raw mut error,
            )
        };
        // SAFETY: `code` and `error` are the call's, untouched since.
        unsafe { finish(code, error) }?;
        Ok(match out.kind {
            CEL_VALUE_BOOL => Value::Bool(out.bool_value != 0),
            CEL_VALUE_STRING => {
                // SAFETY: a string result points at `len` bytes the shim
                // allocated, live until the free below.
                let text = String::from_utf8_lossy(unsafe { ffi_bytes(&out) }).into_owned();
                // SAFETY: shim-allocated, and this is the last use of it.
                unsafe { cel_free(out.data.cast_mut()) };
                Value::String(text)
            }
            _ => Value::Other,
        })
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        // SAFETY: `self.raw` came from `cel_program_new` and is freed once,
        // before `self.engine` releases the engine.
        unsafe { cel_program_free(self.raw) };
    }
}

/// A parsed message, owning its storage. It keeps the engine that parsed
/// it alive.
pub struct Frame {
    raw: *mut CelFrame,
    _engine: Arc<Shared>,
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame").finish_non_exhaustive()
    }
}

// SAFETY: a frame is immutable once parsed.
unsafe impl Send for Frame {}
// SAFETY: as above.
unsafe impl Sync for Frame {}

impl Drop for Frame {
    fn drop(&mut self) {
        // SAFETY: `self.raw` came from `cel_frame_new` and is freed once,
        // before `self.engine` releases the engine.
        unsafe { cel_frame_free(self.raw) };
    }
}
