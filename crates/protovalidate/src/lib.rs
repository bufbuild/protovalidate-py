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
//! evaluated in Rust; custom CEL expressions are compiled and evaluated
//! by [cel-cpp]. The `cel` feature, on by default, brings cel-cpp in.
//! A schema without custom rules can disable it, removing the need to
//! compile any C++.
//!
//! # Usage
//!
//! A [`Validator`] caches compiled rules and is thread-safe; create one and
//! share it. Descriptors are registered serialized, and a message is read in
//! place through a [`Runtime`](protobuf::Runtime) the caller implements over
//! its own Protobuf runtime. Register descriptors up front, then validate
//! messages as they arrive.
//!
//! ```ignore
//! use protovalidate::{Error, Validator};
//!
//! let mut validator = Validator::<MyRuntime>::new();
//! validator.add_file_descriptor_bytes(&file_descriptor_proto)?;
//!
//! match validator.validate_message(type_name, &message, false) {
//!     Ok(()) => { /* valid */ }
//!     Err(Error::Validation(e)) => {
//!         // The message failed to evaluate against rules in the schema.
//!         // e.violations() is a serialized buf.validate.Violations describing
//!         // each failure.
//!     }
//!     // Validation itself failed: unknown type, rules that do not compile
//!     // or evaluate.
//!     Err(e) => return Err(e.into()),
//! }
//! ```
//!
//! Without the `cel` feature, a custom CEL rule is reported as
//! [`Error::Compilation`] when its message type is first validated.
//!
//! A read of the message fails with the runtime's own error type,
//! [`Runtime::Error`](protobuf::Runtime::Error), which comes back as
//! [`Error::Read`]; a native runtime uses [`Infallible`](std::convert::Infallible).
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

pub use error::{DescriptorError, Error, ValidationError};
pub use validator::Validator;
