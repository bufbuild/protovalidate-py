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

//! The CEL backend without the `cel` feature.
//!
//! The types have the shapes the validator uses, but the environment
//! compiles nothing: a custom rule fails to compile, and no [`Program`] or
//! [`Frame`] can be constructed, so the code that would run one is
//! unreachable.

// The shapes mirror the engine's whether or not anything reaches them.
#![allow(dead_code, clippy::unused_self, clippy::unnecessary_wraps)]

/// A scalar, as an expression's `this`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ScalarValue<'a> {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Double(f64),
    String(&'a str),
    Bytes(&'a [u8]),
}

/// What `this` is bound to while a program runs.
#[derive(Clone, Copy, Debug)]
pub(crate) enum This<'a> {
    Scalar(ScalarValue<'a>),
    Message(&'a Frame),
    Field(&'a Frame, i32),
}

/// One expression to compile.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Expression<'a> {
    pub expression: &'a str,
    pub rule_field_number: i32,
}

/// What an expression produced.
#[derive(Debug)]
pub(crate) enum Value {
    Bool(bool),
    String(String),
    Other,
}

/// Why the backend could not do what was asked.
#[derive(Debug)]
pub(crate) enum Error {
    Compilation(String),
    Runtime(String),
    Argument(String),
    Unexpected(String),
}

impl Error {
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Compilation(m) | Self::Runtime(m) | Self::Argument(m) | Self::Unexpected(m) => m,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

/// A compiled program. None exists.
#[derive(Debug)]
pub(crate) enum Program {}

impl Program {
    pub(crate) fn eval(&self, _index: usize, _this: This<'_>) -> Result<Value, Error> {
        match *self {}
    }
}

/// A parsed message. None exists.
#[derive(Debug)]
pub(crate) enum Frame {}

/// An environment that knows the schema but compiles nothing.
#[derive(Debug)]
pub(crate) struct Env;

pub(crate) fn new_env() -> Result<Env, Error> {
    Ok(Env)
}

impl Env {
    pub(crate) fn add_file(&mut self, _bytes: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    pub(crate) fn compile(
        &mut self,
        _rules: Option<(&str, &[u8])>,
        _expressions: &[Expression<'_>],
    ) -> Result<Program, Error> {
        Err(Error::Compilation(
            "custom CEL rules require the `cel` feature of the protovalidate crate".to_owned(),
        ))
    }

    pub(crate) fn frame(&self, _type_name: &str, _payload: &[u8]) -> Result<Frame, Error> {
        Err(Error::Unexpected(
            "no CEL program exists to need a message frame".to_owned(),
        ))
    }
}
