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

//! Functions written in Rust, as CEL calls them: the types an
//! [`Engine::register`](crate::Engine::register)ed function is declared
//! and called with.

use std::borrow::Cow;

/// The type of an argument, as a function is registered. CEL resolves
/// overloads of one name by these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int,
    Uint,
    Double,
    String,
    Bytes,
    List,
}

/// An argument, of the [`Kind`] the function was registered with.
#[derive(Debug)]
pub enum Arg<'a> {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Double(f64),
    /// CEL strings are UTF-8.
    String(Cow<'a, str>),
    Bytes(&'a [u8]),
    /// The list's elements, read up front.
    List(Vec<Element<'a>>),
}

/// A list element. [`Other`](Self::Other) is one that is not a scalar, e.g., a message.
#[derive(Debug)]
pub enum Element<'a> {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Double(f64),
    String(Cow<'a, str>),
    Bytes(&'a [u8]),
    Other,
}

/// A function's implementation: a predicate over its arguments, or the
/// message of the error the expression sees instead of a value.
///
/// protovalidate's functions are all predicates.
pub type NativeFn = fn(&[Arg<'_>]) -> Result<bool, String>;
