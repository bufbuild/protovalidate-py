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

//! The CEL runtime behind the `protovalidate` crate: [cel-cpp], compiled from
//! source with its dependencies, behind a small safe API.
//!
//! An [`Engine`] holds a descriptor pool and compiles expressions into
//! [`Program`]s; a [`Frame`] is a parsed message that programs evaluate
//! against. Both keep the engine they came from alive. Functions written
//! in Rust are added to an engine with [`Engine::register`].
//!
//! This is an internal support crate: its API follows the shim's needs and
//! changes with it.
//!
//! [cel-cpp]: https://github.com/google/cel-cpp

mod engine;
mod ffi;
mod function;

use std::fmt;

pub use engine::{Engine, Frame, Program};
pub use function::{Arg, Element, Kind, NativeFn};

/// A scalar, as an expression's `this`.
#[derive(Clone, Copy, Debug)]
pub enum ScalarValue<'a> {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Double(f64),
    String(&'a str),
    Bytes(&'a [u8]),
}

/// What `this` is bound to while a program runs.
#[derive(Clone, Copy, Debug)]
pub enum This<'a> {
    Scalar(ScalarValue<'a>),
    /// The frame's message.
    Message(&'a Frame),
    /// A field of the frame's message, by number: a list, a map, or a
    /// singular value, as the field's descriptor says.
    Field(&'a Frame, i32),
}

/// One expression to compile.
#[derive(Clone, Copy, Debug)]
pub struct Expression<'a> {
    pub expression: &'a str,
    /// The field of the rules message that `rule` is bound to while the
    /// expression runs, or 0 for none.
    pub rule_field_number: i32,
}

/// What an expression produced, as far as the caller can tell them apart.
#[derive(Debug)]
pub enum Value {
    Bool(bool),
    String(String),
    /// Anything else: a number, a message, a list, null.
    Other,
}

/// Why the runtime could not do what was asked.
#[derive(Debug)]
pub enum Error {
    /// An expression does not compile.
    Compilation(String),
    /// An expression failed while being evaluated.
    Runtime(String),
    /// A bad descriptor, an unknown type, an unparsable payload, or a field
    /// that does not exist.
    Argument(String),
    /// A failure of the runtime itself, or a status the shim does not
    /// define.
    Unexpected(String),
}

impl Error {
    /// The message, whatever the kind.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Compilation(m) | Self::Runtime(m) | Self::Argument(m) | Self::Unexpected(m) => m,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for Error {}
