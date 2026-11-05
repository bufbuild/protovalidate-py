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

//! Protocol Buffer message validation with [protovalidate].
//!
//! Validation rules are written in `.proto` files with the `buf.validate`
//! extensions. This crate evaluates them against messages, supporting the
//! full rule set including custom CEL expressions. The standard rules are
//! evaluated in Rust, and custom CEL expressions by [cel-cpp], which the
//! `cel` feature (on by default) compiles in. A schema without custom
//! rules can disable it and skip the C++ build, in which case `pattern` rules
//! use the `regex` crate instead of RE2.
//!
//! # Usage
//!
//! A [`Validator`] reads messages in place through a
//! [`Runtime`](protobuf::Runtime) that you implement for your Protobuf
//! runtime. Register your descriptors and compile the rules of the message
//! types you will validate, then share the validator and validate messages
//! as they arrive.
//!
//! ```ignore
//! let mut validator = Validator::<MyRuntime>::new();
//! validator.add_file_descriptor_bytes(&file_descriptor_proto)?;
//! validator.compile(type_name, &resolver)?;
//! match validator.validate_message(type_name, &reader, false) {
//!     Ok(()) => { /* valid */ }
//!     Err(Error::Validation(e)) => {
//!         // The message failed validation. `e` prints why, and
//!         // e.encode_violations() is a serialized buf.validate.Violations.
//!         eprintln!("{e}");
//!     }
//!     // Validation could not complete. The type is unknown, its rules do
//!     // not compile or evaluate, or the message could not be read.
//!     Err(e) => return Err(e.into()),
//! }
//! ```
//!
//! Without the `cel` feature, a custom CEL rule is reported as
//! [`Error::Compilation`] when its message type is validated.
//!
//! [protovalidate]: https://buf.build/docs/protovalidate/
//! [cel-cpp]: https://github.com/google/cel-cpp

// protovalidate-deps contains unsafe shims into C++ code, leaving this crate
// as fully safe. Notably, with the cel feature disabled, no unsafe code is
// accessible.
#![forbid(unsafe_code)]

mod cel;
mod descriptors;
pub mod protobuf;
mod rules;

#[allow(
    non_camel_case_types,
    dead_code,
    unused_imports,
    unused_qualifications,
    clippy::pedantic,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::match_single_binding
)]
mod validate {
    //! Generated types for `buf/validate/validate.proto`.
    include!("gen/buf.validate.mod.rs");
}

mod error;
mod validator;
mod violation;

pub use error::{DescriptorError, Error, ValidationError};
pub use validator::{CompiledType, Validator};
