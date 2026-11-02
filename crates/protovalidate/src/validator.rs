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

use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use buffa::Message as _;
use buffa_descriptor::MessageIndex;
use buffa_descriptor::generated::descriptor::{FileDescriptorProto, FileDescriptorSet};

use crate::cel;
use crate::descriptors::{self, Descriptors};
use crate::protobuf::{Reader, Resolver, Runtime};
use crate::rules::build::{self, Builder};
use crate::rules::eval::Walk;
use crate::rules::{ValidatorCache, ValidatorIndex};
use crate::{DescriptorError, Error, ValidationError};

/// Appended to a panic that only a broken build can reach.
const BUILD_BUG: &str = "this is a bug in protovalidate, please report it";

/// A CEL environment with the files of `set` registered.
pub(crate) fn cel_env(set: &FileDescriptorSet) -> cel::Env {
    let mut env = cel::new_env().unwrap_or_else(|error| {
        panic!("could not initialize the CEL runtime: {error}; {BUILD_BUG}")
    });
    for file in &set.file {
        env.add_file(&file.encode_to_vec()).unwrap_or_else(|error| {
            panic!("could not register the buf.validate schema: {error}; {BUILD_BUG}")
        });
    }
    env
}

/// A message type whose rules a [`Validator`] has compiled, as returned by
/// [`Validator::compiled_type`]. Only the validator that returned it can
/// validate with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompiledType {
    /// The [`Validator::id`] of the validator that returned it.
    validator: u64,
    index: ValidatorIndex,
}

/// The [`Validator::id`] of the next validator created.
static NEXT_VALIDATOR_ID: AtomicU64 = AtomicU64::new(0);

/// Validates Protobuf messages against the rules in their descriptors,
/// reading them through the runtime `R`.
///
/// Register your descriptors and compile the rules of the message types you
/// will validate before validating messages.
pub struct Validator<R: Runtime> {
    /// Identifies this validator among all validators in the process, so
    /// that a [`CompiledType`] from another one is rejected.
    id: u64,
    descriptors: Descriptors,
    /// The number of message types in the embedded `buf.validate` schema,
    /// which come first in the pool.
    base_messages: usize,
    env: cel::Env,
    validators: ValidatorCache<R>,
}

impl<R: Runtime> fmt::Debug for Validator<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Validator").finish_non_exhaustive()
    }
}

impl<R: Runtime> Default for Validator<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Runtime> Validator<R> {
    /// Creates a validator.
    ///
    /// Use `add_*` methods to register file descriptors for the messages you will validate.
    #[must_use]
    pub fn new() -> Self {
        let base = descriptors::base_file_set();
        let env = cel_env(&base);
        let descriptors = Descriptors::from_set(base);
        Self {
            id: NEXT_VALIDATOR_ID.fetch_add(1, Ordering::Relaxed),
            base_messages: descriptors.pool.messages().len(),
            descriptors,
            env,
            validators: ValidatorCache::default(),
        }
    }

    /// Registers a serialized `google.protobuf.FileDescriptorProto`.
    ///
    /// A file's imports must be registered before the file itself. Adding a
    /// file whose name is already known is a no-op, so descriptors
    /// may be re-added freely.
    ///
    /// # Errors
    ///
    /// Fails if the bytes do not parse, the file is invalid, or one of its
    /// imports has not been registered.
    pub fn add_file_descriptor_bytes(&mut self, file: &[u8]) -> Result<(), DescriptorError> {
        let proto: FileDescriptorProto = descriptors::decode(file).map_err(DescriptorError::new)?;
        self.add_file_set(FileDescriptorSet {
            file: vec![proto],
            ..Default::default()
        })
    }

    /// Registers every file in a serialized
    /// `google.protobuf.FileDescriptorSet`, such as the output of `buf build
    /// -o` or `protoc --descriptor_set_out`.
    ///
    /// Each file's imports must come before it, which both tools guarantee
    /// when they output a complete image.
    ///
    /// # Errors
    ///
    /// Fails if the set does not parse or one of its files is invalid.
    pub fn add_file_descriptor_set_bytes(&mut self, set: &[u8]) -> Result<(), DescriptorError> {
        let set: FileDescriptorSet = descriptors::decode(set).map_err(DescriptorError::new)?;
        self.add_file_set(set)
    }

    /// Compiles the rules of `type_name` and of the message types reachable
    /// from it, looking up the runtime's types through `resolver`.
    ///
    /// # Errors
    ///
    /// [`Error::Argument`] if the type is not registered, [`Error::Read`] if
    /// the resolver does not know a type, or [`Error::Compilation`] if the
    /// rules of the type or of a type reachable from it do not compile.
    pub fn compile(
        &mut self,
        type_name: &str,
        resolver: &dyn Resolver<R>,
    ) -> Result<(), Error<R::Error>> {
        let index = self.message_index(type_name)?;
        self.compile_index(index, resolver).map_err(Error::Read)?;
        match self.reachable_compile_error(index) {
            Some(error) => Err(Error::Compilation(error)),
            None => Ok(()),
        }
    }

    /// Compiles the rules of every registered message type. The types of the
    /// embedded `buf.validate` schema and its imports are not compiled, even
    /// if their files were registered again.
    ///
    /// # Errors
    ///
    /// [`Error::Read`] if the resolver does not know a type, or
    /// [`Error::Compilation`], naming the types whose rules do not compile.
    /// All other types are still compiled.
    pub fn compile_all(&mut self, resolver: &dyn Resolver<R>) -> Result<(), Error<R::Error>> {
        self.compile_messages(self.base_messages, resolver)
    }

    /// Compiles the rules of every message type from position `first` in the
    /// pool, as [`compile_all`](Self::compile_all) does from the first type
    /// after the embedded schema.
    pub(crate) fn compile_messages(
        &mut self,
        first: usize,
        resolver: &dyn Resolver<R>,
    ) -> Result<(), Error<R::Error>> {
        let pool = &self.descriptors.pool;
        let pending: Vec<MessageIndex> = pool
            .messages()
            .iter()
            .skip(first)
            // A map's entry type is only ever read through its map.
            .filter(|message| {
                message
                    .options()
                    .is_none_or(|options| options.map_entry != Some(true))
            })
            .filter_map(|message| pool.message_index(message.full_name()))
            .filter(|&index| !self.validators.contains(index))
            .collect();
        for &index in &pending {
            self.compile_index(index, resolver).map_err(Error::Read)?;
        }
        let failed: Vec<String> = pending
            .iter()
            .filter_map(|&index| {
                let error = self.compile_error(index)?;
                Some(format!(
                    "{}: {error}",
                    self.descriptors.pool.message(index).full_name()
                ))
            })
            .collect();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(Error::Compilation(failed.join("; ")))
        }
    }

    /// Whether the rules of `type_name` are compiled.
    #[must_use]
    pub fn is_compiled(&self, type_name: &str) -> bool {
        self.descriptors
            .pool
            .message_index(type_name)
            .is_some_and(|index| self.validators.contains(index))
    }

    /// Validates a message read through `reader`.
    ///
    /// `type_name` is the fully qualified message type, without a leading
    /// dot, and its rules must be compiled. With `fail_fast`, validation
    /// stops at the first violation.
    ///
    /// # Errors
    ///
    /// [`Error::Validation`] if the message fails validation,
    /// [`Error::Read`] if the runtime could not read it, [`Error::Argument`]
    /// if the type is not registered or not compiled, [`Error::Compilation`]
    /// if its rules do not compile, or [`Error::Evaluation`] if a rule could
    /// not be evaluated.
    pub fn validate_message(
        &self,
        type_name: &str,
        reader: &dyn Reader<R>,
        fail_fast: bool,
    ) -> Result<(), Error<R::Error>> {
        let message = self.message_index(type_name)?;
        let Some(index) = self.validators.index(message) else {
            return Err(Error::Argument(format!(
                "rules not compiled for message type {type_name}"
            )));
        };
        self.validate_index(index, reader, fail_fast)
    }

    /// Returns the compiled type for `type_name`, which
    /// [`validate_compiled`](Self::validate_compiled) uses to validate
    /// without looking up the name each time. Returns `None` if its rules
    /// are not compiled. As with [`is_compiled`](Self::is_compiled), rules that
    /// failed to compile count as compiled, and validation reports the
    /// error.
    #[must_use]
    pub fn compiled_type(&self, type_name: &str) -> Option<CompiledType> {
        let message = self.descriptors.pool.message_index(type_name)?;
        let index = self.validators.index(message)?;
        Some(CompiledType {
            validator: self.id,
            index,
        })
    }

    /// Validates a message of a type that
    /// [`compiled_type`](Self::compiled_type) returned for this validator.
    /// See [`validate_message`](Self::validate_message).
    ///
    /// # Errors
    ///
    /// The same as [`validate_message`](Self::validate_message), and
    /// [`Error::Argument`] if another validator returned `compiled`.
    pub fn validate_compiled(
        &self,
        compiled: CompiledType,
        reader: &dyn Reader<R>,
        fail_fast: bool,
    ) -> Result<(), Error<R::Error>> {
        if compiled.validator != self.id {
            return Err(Error::Argument(
                "the compiled type is from another validator".to_owned(),
            ));
        }
        self.validate_index(compiled.index, reader, fail_fast)
    }

    fn validate_index(
        &self,
        index: ValidatorIndex,
        reader: &dyn Reader<R>,
        fail_fast: bool,
    ) -> Result<(), Error<R::Error>> {
        let (message_type, validator) = self.validators.validator(index);
        let validator = validator
            .as_ref()
            .map_err(|error| Error::Compilation(error.clone()))?;
        let message = reader
            .message(&validator.message_type)
            .map_err(Error::Read)?;
        let walk = Walk::<R>::new(&self.descriptors, &self.env, &self.validators, fail_fast);
        let violations = walk.validate(validator, &message, message_type)?;
        if violations.is_empty() {
            return Ok(());
        }
        Err(Error::Validation(ValidationError::new(violations)))
    }

    fn message_index(&self, type_name: &str) -> Result<MessageIndex, Error<R::Error>> {
        self.descriptors
            .pool
            .message_index(type_name)
            .ok_or_else(|| Error::Argument(format!("unknown message type: {type_name}")))
    }

    fn add_file_set(&mut self, set: FileDescriptorSet) -> Result<(), DescriptorError> {
        // The CEL pool takes each file serialized separately.
        let files: Vec<Vec<u8>> = set.file.iter().map(buffa::Message::encode_to_vec).collect();
        self.descriptors
            .add_file_set(set)
            .map_err(DescriptorError::new)?;
        // This is not atomic, since the files are added to the validator's
        // pool before the CEL pool. The two pools could only differ for
        // hand-crafted descriptors that protoc, buf and similar tools would
        // not produce. Even then, the error is returned here, and a rule
        // that reaches a type missing from the CEL pool fails with "unknown
        // message type" rather than running against the wrong schema.
        for file in &files {
            self.env
                .add_file(file)
                .map_err(|error| DescriptorError::new(error.to_string()))?;
        }
        Ok(())
    }

    /// The reason a compiled type cannot be validated, if any.
    fn compile_error(&self, index: MessageIndex) -> Option<String> {
        match self.validators.get(index)? {
            Err(error) => Some(error.clone()),
            Ok(validator) => validator
                .field_validators()
                .find_map(|field| field.err().cloned()),
        }
    }

    /// Returns why the rules of `root`, or of a type that validating it can
    /// reach, do not compile. An error in another type names that type.
    fn reachable_compile_error(&self, root: MessageIndex) -> Option<String> {
        let mut pending = vec![self.validators.index(root)?];
        let mut seen = HashSet::new();
        while let Some(index) = pending.pop() {
            if !seen.insert(index) {
                continue;
            }
            let (message, built) = self.validators.validator(index);
            let error = match built {
                Err(error) => Some(error),
                Ok(validator) => {
                    let mut error = None;
                    for field in validator.field_validators() {
                        match field {
                            Err(field_error) => {
                                error = Some(field_error);
                                break;
                            }
                            Ok(field) => {
                                pending.extend(field.nested_type());
                            }
                        }
                    }
                    error
                }
            };
            if let Some(error) = error {
                if message == root {
                    return Some(error.clone());
                }
                let name = self.descriptors.pool.message(message).full_name();
                return Some(format!("{name}: {error}"));
            }
        }
        None
    }

    /// Compiles the rules of `root` and of every type reachable from it
    /// whose rules are not compiled yet.
    fn compile_index(
        &mut self,
        root: MessageIndex,
        resolver: &dyn Resolver<R>,
    ) -> Result<(), R::Error> {
        if self.validators.contains(root) {
            return Ok(());
        }
        let types = build::resolve(&self.descriptors, root, &self.validators, resolver)?;
        Builder::new(&self.descriptors, &mut self.env, types)
            .build_closure(root, &mut self.validators);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
