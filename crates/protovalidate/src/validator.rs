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

use std::fmt;
use std::sync::{Arc, PoisonError, RwLock};

use buffa::Message as _;
use buffa_descriptor::MessageIndex;
use buffa_descriptor::generated::descriptor::{FileDescriptorProto, FileDescriptorSet};

use crate::cel;
use crate::descriptors::{self, Descriptors};
use crate::protobuf::{Reader, Runtime};
use crate::rules::ValidatorCache;
use crate::rules::build::{self, Builder};
use crate::rules::eval::Walk;
use crate::validate::{Violation as ViolationPb, Violations};
use crate::{DescriptorError, Error, ValidationError};

/// The violations as a serialized `buf.validate.Violations`.
fn encode_violations(violations: Vec<ViolationPb>) -> Vec<u8> {
    Violations {
        violations,
        ..Default::default()
    }
    .encode_to_vec()
}

/// A CEL environment with the files of `set` registered.
pub(crate) fn cel_env(set: &FileDescriptorSet) -> cel::Env {
    let mut env = cel::new_env()
        .unwrap_or_else(|error| panic!("could not initialize the CEL runtime: {error}"));
    for file in &set.file {
        env.add_file(&file.encode_to_vec())
            .unwrap_or_else(|error| panic!("could not register the buf.validate schema: {error}"));
    }
    env
}

/// Validates Protobuf messages against the rules in their descriptors.
/// Messages are read through the runtime `R`.
pub struct Validator<R: Runtime> {
    descriptors: Descriptors,
    /// Locked only for calls into CEL, notably it is never locked around
    /// Runtime invocations which may switch threads and reenter.
    env: RwLock<cel::Env>,
    /// Replaced as a whole whenever rules are built.
    cache: RwLock<Arc<ValidatorCache<R>>>,
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
        Self {
            descriptors: Descriptors::from_set(base),
            env: RwLock::new(env),
            cache: RwLock::new(Arc::new(ValidatorCache::default())),
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
        let set = FileDescriptorSet {
            file: vec![proto],
            ..Default::default()
        };
        self.descriptors
            .add_file_set(set)
            .map_err(DescriptorError::new)?;
        self.env
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .add_file(file)
            .map_err(|error| DescriptorError::new(error.to_string()))?;
        Ok(())
    }

    /// Validates a message read in place through a [`Runtime`].
    ///
    /// `type_name` is the fully-qualified message type, without a leading
    /// dot. Its descriptor must already be registered. The message is
    /// serialized, through
    /// [`Message::encode`](crate::protobuf::Message::encode), only if a CEL
    /// rule binds the message itself, a repeated field or a map to `this`.
    /// With `fail_fast` validation stops at the first violation, rather than
    /// accumulating them all.
    ///
    /// # Errors
    ///
    /// A message that breaks its rules returns [`Error::Validation`],
    /// carrying the violations as a serialized `buf.validate.Violations`.
    /// The other variants mean validation itself failed: the runtime could
    /// not read the message ([`Error::Read`]), an unknown type or
    /// unparsable payload ([`Error::Argument`]), rules that do not compile
    /// ([`Error::Compilation`]), or a rule failing to evaluate
    /// ([`Error::Evaluation`]).
    pub fn validate_message(
        &self,
        type_name: &str,
        reader: &dyn Reader<R>,
        fail_fast: bool,
    ) -> Result<(), Error<R::Error>> {
        let violations = self.run(type_name, reader, fail_fast)?;
        if violations.is_empty() {
            return Ok(());
        }
        Err(Error::Validation(ValidationError::new(encode_violations(
            violations,
        ))))
    }

    fn run(
        &self,
        type_name: &str,
        reader: &dyn Reader<R>,
        fail_fast: bool,
    ) -> Result<Vec<ViolationPb>, Error<R::Error>> {
        let index = self.message_index(type_name)?;
        let validators = self.validators(index, reader)?;
        let validator = validators[&index]
            .as_ref()
            .map_err(|error| Error::Compilation(error.clone()))?;
        let message = reader
            .message(&validator.message_type)
            .map_err(Error::Read)?;
        let walk = Walk::<R>::new(&self.descriptors, &self.env, &validators, fail_fast);
        walk.validate(validator, &message, type_name)
    }

    fn message_index(&self, type_name: &str) -> Result<MessageIndex, Error<R::Error>> {
        self.descriptors
            .pool
            .message_index(type_name)
            .ok_or_else(|| Error::Argument(format!("unknown message type: {type_name}")))
    }

    /// Returns the cache, first building the rules of `index` and of every
    /// type reachable from it if they are missing.
    fn validators(
        &self,
        index: MessageIndex,
        reader: &dyn Reader<R>,
    ) -> Result<Arc<ValidatorCache<R>>, Error<R::Error>> {
        let known = self.snapshot();
        if known.contains_key(&index) {
            return Ok(known);
        }
        let types =
            build::resolve(&self.descriptors, index, &known, reader).map_err(Error::Read)?;
        // The rules are built against the snapshot rather than under the
        // cache lock, so other validations are not held up while they
        // compile. Two threads that build the same type at once both merge
        // their result, and the later one wins.
        let mut built = ValidatorCache::default();
        {
            let mut env = self.env.write().unwrap_or_else(PoisonError::into_inner);
            Builder::new(&self.descriptors, &mut env, types)
                .build_closure(index, &known, &mut built);
        }
        let mut cache = self.cache.write().unwrap_or_else(PoisonError::into_inner);
        let mut validators = ValidatorCache::clone(&cache);
        validators.extend(built);
        *cache = Arc::new(validators);
        Ok(Arc::clone(&cache))
    }

    fn snapshot(&self) -> Arc<ValidatorCache<R>> {
        Arc::clone(&self.cache.read().unwrap_or_else(PoisonError::into_inner))
    }
}

#[cfg(test)]
mod tests {
    use super::Validator;
    use crate::protobuf::testing::Untyped;

    #[test]
    fn message_without_rules_is_not_read() {
        let validator = Validator::<Untyped>::new();
        validator
            .validate_message("buf.validate.FieldPathElement", &Untyped, false)
            .expect("a message without rules is valid");
    }
}
