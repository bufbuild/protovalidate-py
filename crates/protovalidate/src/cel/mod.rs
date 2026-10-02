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

//! The CEL backend.
//!
//! Everything the validator needs from a CEL runtime passes through the
//! types exported here: an [`Env`] that compiles rule expressions into a
//! [`Program`], and a [`Frame`] holding a message that programs can be
//! evaluated against. With the `cel` feature they come from
//! `protovalidate-deps`, whose engine wraps cel-cpp, with protovalidate's
//! function [`library`] registered by [`new_env`]. Without it they come
//! from the [`stub`], whose environment compiles nothing, so no program
//! ever exists to run.

#[cfg(feature = "cel")]
mod library;
#[cfg(not(feature = "cel"))]
mod stub;

#[cfg(feature = "cel")]
pub(crate) use protovalidate_deps::{
    Engine as Env, Error, Expression, Frame, Program, ScalarValue, This, Value,
};
#[cfg(not(feature = "cel"))]
pub(crate) use stub::{Env, Error, Expression, Frame, Program, ScalarValue, This, Value, new_env};

/// A CEL environment knowing protovalidate's functions.
///
/// # Errors
///
/// Fails if the runtime cannot be initialized or the library cannot be
/// registered, which indicates a broken build rather than bad input.
#[cfg(feature = "cel")]
pub(crate) fn new_env() -> Result<Env, Error> {
    let mut env = Env::new()?;
    for function in library::FUNCTIONS {
        env.register(function.name, true, function.args, function.call)?;
    }
    Ok(env)
}
