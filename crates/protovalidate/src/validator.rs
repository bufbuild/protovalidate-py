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
                .fields
                .iter()
                .find_map(|field| field.as_ref().err().cloned()),
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
                    for field in &validator.fields {
                        match field {
                            Err(field_error) => {
                                error = Some(field_error);
                                break;
                            }
                            Ok(field) => {
                                pending.extend(build::nested_type(field).map(|n| n.validator));
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
mod tests {
    use super::Validator;
    use crate::protobuf::testing::Untyped;

    #[test]
    fn message_without_rules_is_not_read() {
        let mut validator = Validator::<Untyped>::new();
        validator
            .compile("buf.validate.FieldPathElement", &Untyped)
            .expect("the type compiles");
        validator
            .validate_message("buf.validate.FieldPathElement", &Untyped, false)
            .expect("a message without rules is valid");
    }

    #[test]
    fn uncompiled_type_is_reported() {
        let validator = Validator::<Untyped>::new();
        assert!(!validator.is_compiled("buf.validate.FieldPathElement"));
        let error = validator
            .validate_message("buf.validate.FieldPathElement", &Untyped, false)
            .expect_err("not compiled");
        assert!(matches!(error, crate::Error::Argument(_)), "{error}");
    }

    #[test]
    fn compiled_type_validates_without_its_name() {
        let mut validator = Validator::<Untyped>::new();
        assert!(
            validator
                .compiled_type("buf.validate.FieldPathElement")
                .is_none()
        );
        validator
            .compile("buf.validate.FieldPathElement", &Untyped)
            .expect("the type compiles");
        let compiled = validator
            .compiled_type("buf.validate.FieldPathElement")
            .expect("the type is compiled");
        validator
            .validate_compiled(compiled, &Untyped, false)
            .expect("a message without rules is valid");
        assert!(validator.compiled_type("unknown.Type").is_none());

        let other = Validator::<Untyped>::new();
        let error = other
            .validate_compiled(compiled, &Untyped, false)
            .expect_err("another validator returned the compiled type");
        assert!(matches!(error, crate::Error::Argument(_)), "{error}");
    }

    /// A file with one message whose rules compile and one whose rules do
    /// not. `test.Middle` has a field of type `test.Bad`, and `test.Top` has
    /// a field of type `test.Middle`.
    fn mixed_schema() -> Vec<u8> {
        use buffa::{ExtensionSet as _, Message as _};
        use buffa_descriptor::generated::descriptor::field_descriptor_proto::{Label, Type};
        use buffa_descriptor::generated::descriptor::{
            DescriptorProto, FieldDescriptorProto, FieldOptions, FileDescriptorProto,
            FileDescriptorSet, MessageOptions,
        };

        use crate::validate::__buffa::oneof::field_rules::Type as RulesType;
        use crate::validate::{FIELD, FieldRules, MESSAGE, MessageRules, Rule, StringRules};

        let mut field_options = FieldOptions::default();
        field_options.set_extension(
            &FIELD,
            FieldRules {
                r#type: Some(RulesType::String(Box::new(StringRules {
                    min_len: Some(1),
                    ..Default::default()
                }))),
                ..Default::default()
            },
        );
        let mut message_options = MessageOptions::default();
        message_options.set_extension(
            &MESSAGE,
            MessageRules {
                cel: vec![Rule {
                    id: Some("bad".to_owned()),
                    expression: Some("this.(".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        FileDescriptorSet {
            file: vec![FileDescriptorProto {
                name: Some("test.proto".to_owned()),
                package: Some("test".to_owned()),
                dependency: vec!["buf/validate/validate.proto".to_owned()],
                message_type: vec![
                    DescriptorProto {
                        name: Some("Good".to_owned()),
                        field: vec![FieldDescriptorProto {
                            name: Some("s".to_owned()),
                            number: Some(1),
                            label: Some(Label::LABEL_OPTIONAL),
                            r#type: Some(Type::TYPE_STRING),
                            options: Some(field_options).into(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    DescriptorProto {
                        name: Some("Bad".to_owned()),
                        options: Some(message_options).into(),
                        ..Default::default()
                    },
                    DescriptorProto {
                        name: Some("Middle".to_owned()),
                        field: vec![message_field("bad", 1, ".test.Bad")],
                        ..Default::default()
                    },
                    DescriptorProto {
                        name: Some("Top".to_owned()),
                        field: vec![message_field("middle", 1, ".test.Middle")],
                        ..Default::default()
                    },
                ],
                syntax: Some("proto3".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }
        .encode_to_vec()
    }

    #[test]
    fn compile_all_reports_what_did_not_compile() {
        let mut validator = Validator::<Untyped>::new();
        validator
            .add_file_descriptor_set_bytes(&mixed_schema())
            .expect("the schema registers");
        let error = validator
            .compile_all(&Untyped)
            .expect_err("Bad's rule does not compile");
        assert!(matches!(error, crate::Error::Compilation(_)), "{error}");
        assert!(error.to_string().contains("test.Bad"), "{error}");
        assert!(!error.to_string().contains("test.Good"), "{error}");
        // The types whose rules compiled can still be used.
        assert!(validator.is_compiled("test.Good"));
        let error = validator
            .validate_message("test.Bad", &Untyped, false)
            .expect_err("Bad does not validate");
        assert!(matches!(error, crate::Error::Compilation(_)), "{error}");
    }

    #[test]
    fn compile_reports_rules_that_do_not_compile_deeper_down() {
        let mut validator = Validator::<Untyped>::new();
        validator
            .add_file_descriptor_set_bytes(&mixed_schema())
            .expect("the schema registers");
        let error = validator
            .compile("test.Top", &Untyped)
            .expect_err("Bad's rule does not compile");
        assert!(matches!(error, crate::Error::Compilation(_)), "{error}");
        assert!(
            error
                .to_string()
                .contains("test.Middle: failed to compile embedded type test.Bad"),
            "{error}"
        );
    }

    #[test]
    fn compile_all_skips_the_embedded_schema() {
        let mut validator = Validator::<Untyped>::new();
        validator
            .add_file_descriptor_set_bytes(&read_schema())
            .expect("the schema registers");
        validator.compile_all(&Untyped).expect("the rules compile");
        assert!(validator.is_compiled("test.Outer"));
        assert!(
            !validator.is_compiled("buf.validate.FieldRules"),
            "the embedded schema is not compiled"
        );
    }

    /// Messages for checking which fields validation reads:
    ///
    /// - `test.Outer` has a message field `inner` of `test.Inner`, which has
    ///   rules.
    /// - `test.Holder` has a required message field `empty` of `test.Empty`,
    ///   which has none.
    /// - `test.Choice` has a required oneof of two `test.Inner` fields.
    /// - `test.Nest` has a repeated `test.Inner` field `items` and a
    ///   `map<string, test.Inner>` field `named`.
    /// - `test.Keyed` is the same as `test.Nest`, except that the keys of
    ///   `named` must not be empty.
    /// - `test.Idle` has a oneof of two `test.Empty` fields.
    /// - `test.Needs` has a required message field `inner` of `test.Inner`.
    /// - `test.Bag` has a repeated `test.Empty` field `empties`.
    /// - `test.Bagged` has a message field `bag` of `test.Bag`.
    fn read_schema() -> Vec<u8> {
        use buffa::{ExtensionSet as _, Message as _};
        use buffa_descriptor::generated::descriptor::field_descriptor_proto::{Label, Type};
        use buffa_descriptor::generated::descriptor::{
            DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet,
            OneofDescriptorProto, OneofOptions,
        };

        use crate::validate::__buffa::oneof::field_rules::Type as RulesType;
        use crate::validate::{FieldRules, ONEOF, OneofRules, StringRules};

        let min_len = field_options(FieldRules {
            r#type: Some(RulesType::String(Box::new(StringRules {
                min_len: Some(1),
                ..Default::default()
            }))),
            ..Default::default()
        });
        let required = field_options(FieldRules {
            required: Some(true),
            ..Default::default()
        });
        let mut required_oneof = OneofOptions::default();
        required_oneof.set_extension(
            &ONEOF,
            OneofRules {
                required: Some(true),
                ..Default::default()
            },
        );
        FileDescriptorSet {
            file: vec![FileDescriptorProto {
                name: Some("read.proto".to_owned()),
                package: Some("test".to_owned()),
                dependency: vec!["buf/validate/validate.proto".to_owned()],
                message_type: vec![
                    DescriptorProto {
                        name: Some("Outer".to_owned()),
                        field: vec![message_field("inner", 1, ".test.Inner")],
                        ..Default::default()
                    },
                    DescriptorProto {
                        name: Some("Inner".to_owned()),
                        field: vec![FieldDescriptorProto {
                            name: Some("s".to_owned()),
                            number: Some(1),
                            label: Some(Label::LABEL_OPTIONAL),
                            r#type: Some(Type::TYPE_STRING),
                            options: Some(min_len).into(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    DescriptorProto {
                        name: Some("Holder".to_owned()),
                        field: vec![FieldDescriptorProto {
                            options: Some(required).into(),
                            ..message_field("empty", 1, ".test.Empty")
                        }],
                        ..Default::default()
                    },
                    DescriptorProto {
                        name: Some("Empty".to_owned()),
                        ..Default::default()
                    },
                    nest_message("Nest", false),
                    nest_message("Keyed", true),
                    DescriptorProto {
                        name: Some("Choice".to_owned()),
                        oneof_decl: vec![OneofDescriptorProto {
                            name: Some("kind".to_owned()),
                            options: Some(required_oneof).into(),
                            ..Default::default()
                        }],
                        ..oneof_message("Choice", ".test.Inner")
                    },
                    oneof_message("Idle", ".test.Empty"),
                    DescriptorProto {
                        name: Some("Needs".to_owned()),
                        field: vec![FieldDescriptorProto {
                            options: Some(field_options(FieldRules {
                                required: Some(true),
                                ..Default::default()
                            }))
                            .into(),
                            ..message_field("inner", 1, ".test.Inner")
                        }],
                        ..Default::default()
                    },
                ]
                .into_iter()
                .chain(bag_messages())
                .collect(),
                syntax: Some("proto3".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }
        .encode_to_vec()
    }

    /// `test.Bag` and `test.Bagged`, from [`read_schema`].
    fn bag_messages() -> [buffa_descriptor::generated::descriptor::DescriptorProto; 2] {
        use buffa_descriptor::generated::descriptor::field_descriptor_proto::Label;
        use buffa_descriptor::generated::descriptor::{DescriptorProto, FieldDescriptorProto};

        [
            DescriptorProto {
                name: Some("Bag".to_owned()),
                field: vec![FieldDescriptorProto {
                    label: Some(Label::LABEL_REPEATED),
                    ..message_field("empties", 1, ".test.Empty")
                }],
                ..Default::default()
            },
            DescriptorProto {
                name: Some("Bagged".to_owned()),
                field: vec![message_field("bag", 1, ".test.Bag")],
                ..Default::default()
            },
        ]
    }

    /// A message with a oneof `kind` of two fields, `a` and `b`, of type
    /// `member_type`.
    fn oneof_message(
        name: &str,
        member_type: &str,
    ) -> buffa_descriptor::generated::descriptor::DescriptorProto {
        use buffa_descriptor::generated::descriptor::{
            DescriptorProto, FieldDescriptorProto, OneofDescriptorProto,
        };

        DescriptorProto {
            name: Some(name.to_owned()),
            field: vec![
                FieldDescriptorProto {
                    oneof_index: Some(0),
                    ..message_field("a", 1, member_type)
                },
                FieldDescriptorProto {
                    oneof_index: Some(0),
                    ..message_field("b", 2, member_type)
                },
            ],
            oneof_decl: vec![OneofDescriptorProto {
                name: Some("kind".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn message_field(
        name: &str,
        number: i32,
        type_name: &str,
    ) -> buffa_descriptor::generated::descriptor::FieldDescriptorProto {
        use buffa_descriptor::generated::descriptor::FieldDescriptorProto;
        use buffa_descriptor::generated::descriptor::field_descriptor_proto::{Label, Type};

        FieldDescriptorProto {
            name: Some(name.to_owned()),
            number: Some(number),
            label: Some(Label::LABEL_OPTIONAL),
            r#type: Some(Type::TYPE_MESSAGE),
            type_name: Some(type_name.to_owned()),
            ..Default::default()
        }
    }

    /// `test.Nest` or `test.Keyed`, from [`read_schema`]. The map `named` has
    /// map rules with rules for its values. With `key_rules`, they also
    /// require non-empty keys. Otherwise, they read nothing themselves.
    fn nest_message(
        name: &str,
        key_rules: bool,
    ) -> buffa_descriptor::generated::descriptor::DescriptorProto {
        use buffa_descriptor::generated::descriptor::field_descriptor_proto::{Label, Type};
        use buffa_descriptor::generated::descriptor::{
            DescriptorProto, FieldDescriptorProto, MessageOptions,
        };

        use crate::validate::__buffa::oneof::field_rules::Type as RulesType;
        use crate::validate::{FieldRules, MapRules, StringRules};

        let keys = key_rules.then(|| FieldRules {
            r#type: Some(RulesType::String(Box::new(StringRules {
                min_len: Some(1),
                ..Default::default()
            }))),
            ..Default::default()
        });
        let map_rules = field_options(FieldRules {
            r#type: Some(RulesType::Map(Box::new(MapRules {
                keys: keys.into(),
                values: Some(FieldRules::default()).into(),
                ..Default::default()
            }))),
            ..Default::default()
        });

        DescriptorProto {
            name: Some(name.to_owned()),
            field: vec![
                FieldDescriptorProto {
                    label: Some(Label::LABEL_REPEATED),
                    ..message_field("items", 1, ".test.Inner")
                },
                FieldDescriptorProto {
                    label: Some(Label::LABEL_REPEATED),
                    options: Some(map_rules).into(),
                    ..message_field("named", 2, &format!(".test.{name}.NamedEntry"))
                },
            ],
            nested_type: vec![DescriptorProto {
                name: Some("NamedEntry".to_owned()),
                field: vec![
                    FieldDescriptorProto {
                        name: Some("key".to_owned()),
                        number: Some(1),
                        label: Some(Label::LABEL_OPTIONAL),
                        r#type: Some(Type::TYPE_STRING),
                        ..Default::default()
                    },
                    message_field("value", 2, ".test.Inner"),
                ],
                options: Some(MessageOptions {
                    map_entry: Some(true),
                    ..Default::default()
                })
                .into(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn field_options(
        rules: crate::validate::FieldRules,
    ) -> buffa_descriptor::generated::descriptor::FieldOptions {
        use buffa::ExtensionSet as _;

        let mut options = buffa_descriptor::generated::descriptor::FieldOptions::default();
        options.set_extension(&crate::validate::FIELD, rules);
        options
    }

    /// A runtime whose messages have either every field set or none. It
    /// counts presence and oneof checks, and fails the test if a field's
    /// value is read. Its oneofs can report a member that the validator does
    /// not know of.
    mod probe {
        use std::cell::Cell;
        use std::convert::Infallible;
        use std::ops::ControlFlow;

        use crate::protobuf::{
            Field, List, Map, Message, Oneof, Reader, Resolver, Runtime, Val, ValueKind,
        };

        /// A field number that no oneof member in the schema has.
        pub(super) const UNKNOWN_MEMBER: u32 = 99;

        pub(super) struct Probe {
            set: bool,
            unknown_member: bool,
            pub(super) has_calls: Cell<usize>,
            pub(super) oneof_calls: Cell<usize>,
        }

        impl Probe {
            pub(super) fn new(set: bool) -> Self {
                Self {
                    set,
                    unknown_member: false,
                    has_calls: Cell::new(0),
                    oneof_calls: Cell::new(0),
                }
            }

            /// A probe whose oneofs have a member set that the validator
            /// does not know of.
            pub(super) fn unknown_member() -> Self {
                Self {
                    unknown_member: true,
                    ..Self::new(false)
                }
            }
        }

        impl Runtime for Probe {
            type Error = Infallible;
            type MessageType = ();
            type FieldType = ();
            type Message<'a> = ProbeMessage<'a>;
            type List<'a> = Never;
            type Map<'a> = Never;
        }

        impl Resolver<Probe> for Probe {
            fn resolve(&self, _full_name: &str) -> Result<(), Infallible> {
                Ok(())
            }

            fn resolve_field(&self, _message_type: &(), _number: u32) -> Result<(), Infallible> {
                Ok(())
            }
        }

        impl Reader<Probe> for Probe {
            fn message<'a>(
                &'a self,
                _message_type: &'a (),
            ) -> Result<ProbeMessage<'a>, Infallible> {
                Ok(ProbeMessage(self))
            }
        }

        pub(super) struct ProbeMessage<'a>(&'a Probe);

        impl Message<Probe> for ProbeMessage<'_> {
            fn has(&self, _field: &Field<Probe>) -> Result<bool, Infallible> {
                self.0.has_calls.set(self.0.has_calls.get() + 1);
                Ok(self.0.set)
            }

            fn get<'f, K: ValueKind<Probe>>(
                &'f self,
                field: &'f Field<Probe>,
            ) -> Result<Option<K::Value<'f>>, Infallible> {
                panic!("field {} is read", field.name())
            }

            fn which_oneof(&self, oneof: &Oneof<Probe>) -> Result<Option<u32>, Infallible> {
                self.0.oneof_calls.set(self.0.oneof_calls.get() + 1);
                Ok(if self.0.unknown_member {
                    Some(UNKNOWN_MEMBER)
                } else if self.0.set {
                    oneof.members().first().map(Field::number)
                } else {
                    None
                })
            }

            #[expect(unreachable_code, reason = "the hidden type must be named")]
            fn encode(&self) -> Result<impl AsRef<[u8]>, Infallible> {
                Ok::<&[u8], _>(unreachable!("the test does not encode a message"))
            }
        }

        /// No list or map is ever read.
        pub(super) enum Never {}

        impl List<Probe> for Never {
            fn len(&self) -> Result<usize, Infallible> {
                match *self {}
            }

            fn get(&self, _index: usize) -> Result<Option<Val<'_, Probe>>, Infallible> {
                match *self {}
            }
        }

        impl Map<Probe> for Never {
            fn len(&self) -> Result<usize, Infallible> {
                match *self {}
            }

            fn for_each<B, F>(&self, _f: F) -> Result<ControlFlow<B>, Infallible>
            where
                F: for<'k> FnMut(crate::protobuf::Key<'k, Probe>, Val<'k, Probe>) -> ControlFlow<B>,
            {
                match *self {}
            }
        }
    }

    /// Validates a message of type `type_name` through `probe`.
    fn probe(type_name: &str, probe: &probe::Probe) -> Result<(), crate::Error> {
        let mut validator = Validator::<probe::Probe>::new();
        validator
            .add_file_descriptor_set_bytes(&read_schema())
            .expect("the schema registers");
        validator
            .compile(type_name, probe)
            .expect("the rules compile");
        validator.validate_message(type_name, probe, false)
    }

    #[test]
    fn unset_message_field_is_not_read() {
        probe("test.Outer", &probe::Probe::new(false)).expect("an unset message field is valid");
    }

    #[test]
    fn field_of_type_without_rules_is_not_read() {
        probe("test.Holder", &probe::Probe::new(true))
            .expect("a set required field whose type has no rules is valid");
    }

    #[test]
    fn unset_required_message_is_reported_without_reading_it() {
        let unset = probe::Probe::new(false);
        let error = probe("test.Needs", &unset).expect_err("the required field is unset");
        let crate::Error::Validation(error) = error else {
            panic!("{error}");
        };
        assert_eq!(error.violations().len(), 1, "{error}");
        assert_eq!(error.violations()[0].field_path(), "inner");
        assert_eq!(error.violations()[0].rule_id(), "required");
        assert_eq!(unset.has_calls.get(), 1, "presence is checked once");
    }

    #[test]
    fn oneof_of_members_without_rules_is_not_asked() {
        let set = probe::Probe::new(true);
        probe("test.Idle", &set).expect("members whose type has no rules are valid");
        assert_eq!(set.oneof_calls.get(), 0, "the oneof validates nothing");
        assert_eq!(set.has_calls.get(), 0, "no member is checked on its own");
    }

    #[test]
    fn fields_are_planned_by_what_reads_them() {
        use crate::rules::Plan;

        let mut validator = Validator::<probe::Probe>::new();
        validator
            .add_file_descriptor_set_bytes(&read_schema())
            .expect("the schema registers");
        let probe = probe::Probe::new(false);
        for type_name in [
            "test.Outer",
            "test.Holder",
            "test.Nest",
            "test.Choice",
            "test.Idle",
            "test.Bagged",
        ] {
            validator
                .compile(type_name, &probe)
                .expect("the rules compile");
        }
        let compiled = |type_name: &str| {
            let message = validator.message_index(type_name).expect("registered");
            validator
                .validators
                .get(message)
                .expect("compiled")
                .as_ref()
                .expect("the rules compile")
                .clone()
        };
        let plans = |type_name: &str| {
            compiled(type_name)
                .fields
                .iter()
                .map(|field| field.as_ref().expect("the field compiles").plan)
                .collect::<Vec<_>>()
        };
        assert_eq!(plans("test.Inner"), [Plan::Scalar]);
        assert_eq!(plans("test.Outer"), [Plan::Message]);
        assert_eq!(plans("test.Holder"), [Plan::Presence]);
        assert_eq!(plans("test.Nest"), [Plan::Generic, Plan::Generic]);

        assert_eq!(
            &*compiled("test.Holder").plain,
            [0],
            "a required field is checked"
        );
        assert_eq!(&*compiled("test.Nest").plain, [0, 1]);
        let choice = compiled("test.Choice");
        assert!(
            choice.plain.is_empty(),
            "members are validated through the oneof"
        );
        assert_eq!(&*choice.oneofs[0].by_number, [(1, 0), (2, 1)]);
        let idle = compiled("test.Idle");
        assert!(
            idle.oneofs.is_empty(),
            "a oneof that validates nothing is dropped"
        );
        assert!(
            idle.plain.is_empty(),
            "members without rules are not validated"
        );
        for type_name in ["test.Bag", "test.Bagged"] {
            assert!(
                compiled(type_name).is_empty(),
                "{type_name} only holds messages without rules"
            );
            assert!(
                compiled(type_name).plain.is_empty(),
                "{type_name} validates nothing"
            );
        }
    }

    /// A runtime for `test.Nest` whose map `named` has one entry, `"k"`,
    /// whose value is a valid or invalid `test.Inner`. It counts reads of
    /// the key and of the map's size.
    mod keys {
        use std::borrow::Cow;
        use std::cell::Cell;
        use std::ops::ControlFlow;

        use crate::protobuf::{
            Field, Key, List, Map, Message, Reader, Resolver, Runtime, Val, ValueKind, ValueKindId,
        };

        pub(super) struct Keys {
            valid: bool,
            fails: bool,
            pub(super) reads: Cell<usize>,
            pub(super) sizes: Cell<usize>,
        }

        impl Keys {
            pub(super) fn new(valid: bool, fails: bool) -> Self {
                Self {
                    valid,
                    fails,
                    reads: Cell::new(0),
                    sizes: Cell::new(0),
                }
            }
        }

        impl Runtime for Keys {
            type Error = String;
            type MessageType = ();
            type FieldType = ();
            type Message<'a> = KeysMessage<'a>;
            type List<'a> = Entries<'a>;
            type Map<'a> = Entries<'a>;
        }

        impl Resolver<Keys> for Keys {
            fn resolve(&self, _full_name: &str) -> Result<(), String> {
                Ok(())
            }

            fn resolve_field(&self, _message_type: &(), _number: u32) -> Result<(), String> {
                Ok(())
            }
        }

        impl Reader<Keys> for Keys {
            fn message<'a>(&'a self, _message_type: &'a ()) -> Result<KeysMessage<'a>, String> {
                Ok(KeysMessage::Nest(self))
            }
        }

        /// `test.Nest`, or the `test.Inner` in its map.
        pub(super) enum KeysMessage<'a> {
            Nest(&'a Keys),
            Inner(&'a Keys),
        }

        fn unread<T>(field: &Field<Keys>) -> Result<Option<T>, String> {
            panic!("field {} is read", field.name())
        }

        impl Message<Keys> for KeysMessage<'_> {
            fn has(&self, _field: &Field<Keys>) -> Result<bool, String> {
                Ok(false)
            }
            fn get<'f, K: ValueKind<Keys>>(
                &'f self,
                field: &'f Field<Keys>,
            ) -> Result<Option<K::Value<'f>>, String> {
                let value = match (K::ID, self) {
                    (ValueKindId::Str, Self::Inner(keys)) => {
                        Val::String(Cow::Borrowed(if keys.valid { "a" } else { "" }))
                    }
                    (ValueKindId::List, _) => Val::List(Entries(None)),
                    (ValueKindId::Map, Self::Nest(keys)) => Val::Map(Entries(Some(keys))),
                    _ => return unread(field),
                };
                Ok(K::from_val(value))
            }
            #[expect(unreachable_code, reason = "the hidden type must be named")]
            fn encode(&self) -> Result<impl AsRef<[u8]>, String> {
                Ok::<&[u8], _>(unreachable!("the test does not encode a message"))
            }
        }

        /// The map's one entry, or an empty list.
        pub(super) struct Entries<'a>(Option<&'a Keys>);

        impl List<Keys> for Entries<'_> {
            fn len(&self) -> Result<usize, String> {
                Ok(0)
            }
            fn get(&self, _index: usize) -> Result<Option<Val<'_, Keys>>, String> {
                Ok(None)
            }
        }

        impl Map<Keys> for Entries<'_> {
            fn len(&self) -> Result<usize, String> {
                if let Some(keys) = self.0 {
                    keys.sizes.set(keys.sizes.get() + 1);
                }
                Ok(usize::from(self.0.is_some()))
            }
            fn for_each<B, F>(&self, mut f: F) -> Result<ControlFlow<B>, String>
            where
                F: for<'k> FnMut(Key<'k, Keys>, Val<'k, Keys>) -> ControlFlow<B>,
            {
                let Some(keys) = self.0 else {
                    return Ok(ControlFlow::Continue(()));
                };
                let key = || {
                    keys.reads.set(keys.reads.get() + 1);
                    if keys.fails {
                        Err("the key cannot be read".to_owned())
                    } else {
                        Ok(Val::String(Cow::Borrowed("k")))
                    }
                };
                Ok(f(&key, Val::Message(KeysMessage::Inner(keys))))
            }
        }
    }

    /// Validates `test.Nest` through `keys`.
    fn validate_keys(keys: &keys::Keys) -> Result<(), crate::Error<String>> {
        validate_keys_of("test.Nest", keys)
    }

    /// Validates a message of type `type_name` through `keys`.
    fn validate_keys_of(type_name: &str, keys: &keys::Keys) -> Result<(), crate::Error<String>> {
        let mut validator = Validator::<keys::Keys>::new();
        validator
            .add_file_descriptor_set_bytes(&read_schema())
            .expect("the schema registers");
        validator
            .compile(type_name, keys)
            .expect("the rules compile");
        validator.validate_message(type_name, keys, false)
    }

    #[test]
    fn map_keys_are_read_only_for_violations() {
        let valid = keys::Keys::new(true, false);
        validate_keys(&valid).expect("the map's value is valid");
        assert_eq!(valid.reads.get(), 0, "a valid entry's key is not read");

        let invalid = keys::Keys::new(false, false);
        let error = validate_keys(&invalid).expect_err("the map's value is invalid");
        let crate::Error::Validation(error) = error else {
            panic!("{error}");
        };
        assert_eq!(error.violations()[0].field_path(), "named[\"k\"].s");
        assert_eq!(invalid.reads.get(), 1, "the key is read for the subscript");
    }

    #[test]
    fn map_key_with_rules_is_read_once() {
        let invalid = keys::Keys::new(false, false);
        let error =
            validate_keys_of("test.Keyed", &invalid).expect_err("the map's value is invalid");
        let crate::Error::Validation(error) = error else {
            panic!("{error}");
        };
        assert_eq!(error.violations()[0].field_path(), "named[\"k\"].s");
        assert_eq!(
            invalid.reads.get(),
            1,
            "the key read for its rules is reused for the subscript"
        );
    }

    #[test]
    fn map_size_is_not_read_without_rules_for_it() {
        let keys = keys::Keys::new(true, false);
        validate_keys(&keys).expect("the map's value is valid");
        assert_eq!(
            keys.sizes.get(),
            0,
            "neither map rules without pairs bounds nor presence read the size"
        );
    }

    #[test]
    fn unreadable_map_key_is_reported() {
        let error = validate_keys(&keys::Keys::new(false, true)).expect_err("the key fails");
        assert!(matches!(error, crate::Error::Read(_)), "{error}");
    }

    #[test]
    fn oneof_is_asked_once() {
        let unset = probe::Probe::new(false);
        let error = probe("test.Choice", &unset).expect_err("the required oneof is unset");
        let crate::Error::Validation(error) = error else {
            panic!("{error}");
        };
        assert_eq!(error.violations().len(), 1, "{error}");
        assert_eq!(unset.oneof_calls.get(), 1, "the oneof is asked once");
        assert_eq!(unset.has_calls.get(), 0, "no member is checked on its own");
    }

    #[test]
    fn unknown_oneof_member_sets_the_oneof() {
        probe("test.Choice", &probe::Probe::unknown_member())
            .expect("a required oneof set to a member without rules is valid");
    }

    #[test]
    fn oneof_defaults_to_checking_members() {
        use crate::protobuf::{Field, Reader as _, which_oneof_by_has};

        let mut validator = Validator::<probe::Probe>::new();
        validator
            .add_file_descriptor_set_bytes(&read_schema())
            .expect("the schema registers");
        let unset = probe::Probe::new(false);
        validator
            .compile("test.Choice", &unset)
            .expect("the rules compile");
        let message = validator.message_index("test.Choice").expect("registered");
        let oneof = &validator
            .validators
            .get(message)
            .expect("compiled")
            .as_ref()
            .expect("the rules compile")
            .oneofs[0]
            .oneof;
        assert_eq!(oneof.name(), "kind");
        assert_eq!(
            oneof
                .members()
                .iter()
                .map(Field::number)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        let message = unset.message(&()).expect("a message");
        assert_eq!(
            which_oneof_by_has(&message, oneof).expect("infallible"),
            None
        );
        assert_eq!(unset.has_calls.get(), 2, "one presence check per member");
    }
}
