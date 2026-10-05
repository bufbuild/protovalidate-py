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

//! The C ABI of `shim/cel_shim.h`, declaration for declaration. Everything
//! here is raw; the safe API over it is the rest of the crate.

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

pub(crate) const CEL_OK: c_int = 0;
pub(crate) const CEL_ERR_COMPILATION: c_int = 1;
pub(crate) const CEL_ERR_RUNTIME: c_int = 2;
pub(crate) const CEL_ERR_ARGUMENT: c_int = 3;

pub(crate) const CEL_VALUE_OTHER: i32 = 0;
pub(crate) const CEL_VALUE_BOOL: i32 = 1;
pub(crate) const CEL_VALUE_INT: i32 = 2;
pub(crate) const CEL_VALUE_UINT: i32 = 3;
pub(crate) const CEL_VALUE_DOUBLE: i32 = 4;
pub(crate) const CEL_VALUE_STRING: i32 = 5;
pub(crate) const CEL_VALUE_BYTES: i32 = 6;
pub(crate) const CEL_VALUE_LIST: i32 = 7;

pub(crate) const CEL_THIS_SCALAR: c_int = 0;
pub(crate) const CEL_THIS_MESSAGE: c_int = 1;
pub(crate) const CEL_THIS_FIELD: c_int = 2;

/// Opaque engine handle; see `cel_engine` in `shim/cel_shim.h`.
#[repr(C)]
pub(crate) struct CelEngine {
    _opaque: [u8; 0],
}

/// Opaque compiled program; see `cel_program` in `shim/cel_shim.h`.
#[repr(C)]
pub(crate) struct CelProgram {
    _opaque: [u8; 0],
}

/// Opaque parsed message frame; see `cel_frame` in `shim/cel_shim.h`.
#[repr(C)]
pub(crate) struct CelFrame {
    _opaque: [u8; 0],
}

/// A value; see `cel_value` in `shim/cel_shim.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct CelValue {
    pub kind: i32,
    pub bool_value: i32,
    pub int_value: i64,
    pub uint_value: u64,
    pub double_value: f64,
    pub data: *const u8,
    pub len: usize,
    pub list: *const CelList,
}

/// Nothing set: `CEL_VALUE_OTHER` with every field zero.
impl Default for CelValue {
    fn default() -> Self {
        Self {
            kind: CEL_VALUE_OTHER,
            bool_value: 0,
            int_value: 0,
            uint_value: 0,
            double_value: 0.0,
            data: ptr::null(),
            len: 0,
            list: ptr::null(),
        }
    }
}

/// One expression to compile; see `cel_rule` in `shim/cel_shim.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct CelRule {
    pub expression: *const c_char,
    pub expression_len: usize,
    pub rule_field_number: i32,
}

/// Opaque list passed to a registered function; see `cel_list` in
/// `shim/cel_shim.h`.
#[repr(C)]
pub(crate) struct CelList {
    _opaque: [u8; 0],
}

/// A function registered with `cel_engine_register`; see `cel_native_fn` in
/// `shim/cel_shim.h`.
pub(crate) type CelNativeFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    args: *const CelValue,
    len: usize,
    out: *mut c_int,
    error: *mut *mut c_char,
) -> c_int;

unsafe extern "C" {
    pub(crate) fn cel_engine_new(error: *mut *mut c_char) -> *mut CelEngine;
    pub(crate) fn cel_engine_free(engine: *mut CelEngine);
    pub(crate) fn cel_engine_register(
        engine: *mut CelEngine,
        name: *const c_char,
        name_len: usize,
        receiver_style: c_int,
        arg_kinds: *const i32,
        arity: usize,
        function: CelNativeFn,
        ctx: *mut c_void,
        error: *mut *mut c_char,
    ) -> c_int;
    pub(crate) fn cel_list_len(list: *const CelList) -> usize;
    pub(crate) fn cel_list_get(list: *const CelList, index: usize, out: *mut CelValue);
    pub(crate) fn cel_string_new(data: *const c_char, len: usize) -> *mut c_char;
    pub(crate) fn cel_engine_add_file(
        engine: *mut CelEngine,
        file_descriptor_proto: *const u8,
        len: usize,
        error: *mut *mut c_char,
    ) -> c_int;

    pub(crate) fn cel_program_new(
        engine: *mut CelEngine,
        rules_type_name: *const c_char,
        rules_type_name_len: usize,
        rules: *const u8,
        rules_len: usize,
        exprs: *const CelRule,
        exprs_len: usize,
        out: *mut *mut CelProgram,
        error: *mut *mut c_char,
    ) -> c_int;
    pub(crate) fn cel_program_free(program: *mut CelProgram);

    pub(crate) fn cel_frame_new(
        engine: *mut CelEngine,
        type_name: *const c_char,
        type_name_len: usize,
        payload: *const u8,
        payload_len: usize,
        out: *mut *mut CelFrame,
        error: *mut *mut c_char,
    ) -> c_int;
    pub(crate) fn cel_frame_free(frame: *mut CelFrame);

    pub(crate) fn cel_program_eval(
        program: *const CelProgram,
        index: usize,
        this_kind: c_int,
        scalar: *const CelValue,
        frame: *const CelFrame,
        field_number: i32,
        out: *mut CelValue,
        error: *mut *mut c_char,
    ) -> c_int;

    pub(crate) fn cel_free(ptr: *mut u8);
}
