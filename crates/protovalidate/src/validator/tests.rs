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

    use crate::protobuf::{Field, List, Map, Message, Oneof, Reader, Resolver, Runtime, Val};

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
        fn message<'a>(&'a self, _message_type: &'a ()) -> Result<ProbeMessage<'a>, Infallible> {
            Ok(ProbeMessage(self))
        }
    }

    pub(super) struct ProbeMessage<'a>(&'a Probe);

    impl Message<Probe> for ProbeMessage<'_> {
        fn has(&self, _field: &Field<Probe>) -> Result<bool, Infallible> {
            self.0.has_calls.set(self.0.has_calls.get() + 1);
            Ok(self.0.set)
        }

        crate::protobuf::testing::unread_fields!(Probe);

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
fn fields_are_shaped_by_what_reads_them() {
    use crate::rules::FieldValidator;

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
    let shapes = |type_name: &str| {
        compiled(type_name)
            .fields
            .iter()
            .map(|field| match field.as_ref().expect("the field compiles") {
                FieldValidator::Presence(_) => "presence",
                FieldValidator::Scalar(..) => "scalar",
                FieldValidator::Message(..) => "message",
                FieldValidator::Value(..) => "value",
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(shapes("test.Inner"), ["scalar"]);
    assert_eq!(shapes("test.Outer"), ["message"]);
    assert_eq!(
        shapes("test.Holder"),
        ["presence"],
        "a required field is checked"
    );
    assert_eq!(shapes("test.Nest"), ["value", "value"]);

    let choice = compiled("test.Choice");
    assert!(
        choice.fields.is_empty(),
        "members are validated through the oneof"
    );
    let members: Vec<u32> = choice.oneofs[0]
        .members
        .iter()
        .map(|(number, _)| *number)
        .collect();
    assert_eq!(members, [1, 2]);
    let idle = compiled("test.Idle");
    assert!(
        idle.oneofs.is_empty(),
        "a oneof that validates nothing is dropped"
    );
    assert!(
        idle.fields.is_empty(),
        "members without rules are not validated"
    );
    for type_name in ["test.Bag", "test.Bagged"] {
        assert!(
            compiled(type_name).is_empty(),
            "{type_name} only holds messages without rules"
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

    use crate::protobuf::{Field, Key, List, Map, Message, Reader, Resolver, Runtime, Val};

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
        fn get_bool(&self, field: &Field<Keys>) -> Result<Option<bool>, String> {
            unread(field)
        }
        fn get_int(&self, field: &Field<Keys>) -> Result<Option<i64>, String> {
            unread(field)
        }
        fn get_uint(&self, field: &Field<Keys>) -> Result<Option<u64>, String> {
            unread(field)
        }
        fn get_double(&self, field: &Field<Keys>) -> Result<Option<f64>, String> {
            unread(field)
        }
        fn get_enum(&self, field: &Field<Keys>) -> Result<Option<i32>, String> {
            unread(field)
        }
        fn get_str<'f>(&'f self, field: &'f Field<Keys>) -> Result<Option<Cow<'f, str>>, String> {
            match self {
                Self::Inner(keys) => Ok(Some(Cow::Borrowed(if keys.valid { "a" } else { "" }))),
                Self::Nest(_) => unread(field),
            }
        }
        fn get_bytes<'f>(
            &'f self,
            field: &'f Field<Keys>,
        ) -> Result<Option<Cow<'f, [u8]>>, String> {
            unread(field)
        }
        fn get_message<'f>(
            &'f self,
            field: &'f Field<Keys>,
        ) -> Result<Option<KeysMessage<'f>>, String> {
            unread(field)
        }
        fn get_list<'f>(&'f self, _field: &'f Field<Keys>) -> Result<Option<Entries<'f>>, String> {
            Ok(Some(Entries(None)))
        }
        fn get_map<'f>(&'f self, field: &'f Field<Keys>) -> Result<Option<Entries<'f>>, String> {
            match self {
                Self::Nest(keys) => Ok(Some(Entries(Some(keys)))),
                Self::Inner(_) => unread(field),
            }
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
    let error = validate_keys_of("test.Keyed", &invalid).expect_err("the map's value is invalid");
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
