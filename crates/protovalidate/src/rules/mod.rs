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

//! Compiled validation rules, and their evaluation against messages.
//!
//! [`build`] turns the `buf.validate` options of a message type into a
//! [`PendingValidator`](pending::PendingValidator), which is finalized
//! into a [`MessageValidator`] once the rules of every type it refers to
//! are known. A message or field whose rules do not compile is the error
//! instead, reported when the walk reaches it. [`eval`] walks a message
//! with the validators, reading it through a
//! [`Runtime`](crate::protobuf::Runtime).

pub(crate) mod build;
pub(crate) mod eval;
pub(crate) mod pending;
pub(crate) mod standard;

use std::collections::HashMap;
use std::sync::Arc;

use buffa_descriptor::MessageIndex;

use crate::cel;
use crate::protobuf::{Field, Oneof, Runtime, Scalar, Singular};
use crate::validate::FieldPathElement;
use standard::Checks;

/// A CEL expression, and what it reports when it fails.
pub(crate) struct CelRule {
    pub id: String,
    pub message: String,
    pub expression: String,
    /// The field of the rules message `rule` is bound to while the
    /// expression runs, or 0 for none.
    pub rule_field_number: i32,
    /// The rule path, from `FieldRules` down: `[FieldRules.cel[i]]` for a
    /// custom rule, `[FieldRules.string, (pkg.rule)]` for a predefined one,
    /// empty for a message-level one.
    pub rule_path: Vec<FieldPathElement>,
}

/// CEL expressions compiled together, evaluated against one `this`.
pub(crate) struct CelPrograms {
    pub program: cel::Program,
    pub rules: Vec<CelRule>,
}

/// A message type's validator, or why its rules did not compile.
pub(crate) type Built<R> = Result<Arc<MessageValidator<R>>, String>;

/// The position of a message type's validator in a [`ValidatorCache`].
/// Rules use it to refer to the validators of nested message types, much
/// as a `MessageIndex` identifies a descriptor in its pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ValidatorIndex(u32);

/// The compiled rules of every message type validated so far. Validators
/// are never removed, so a [`ValidatorIndex`] remains valid.
pub(crate) struct ValidatorCache<R: Runtime> {
    /// Each validator at its index, along with the message type it
    /// validates.
    validators: Vec<(MessageIndex, Built<R>)>,
    indices: HashMap<MessageIndex, ValidatorIndex, foldhash::fast::RandomState>,
}

impl<R: Runtime> Default for ValidatorCache<R> {
    fn default() -> Self {
        Self {
            validators: Vec::new(),
            indices: HashMap::default(),
        }
    }
}

impl<R: Runtime> ValidatorCache<R> {
    /// Whether the rules of `message` are compiled.
    pub(crate) fn contains(&self, message: MessageIndex) -> bool {
        self.indices.contains_key(&message)
    }

    /// The index of the validator of `message`, if its rules are compiled.
    pub(crate) fn index(&self, message: MessageIndex) -> Option<ValidatorIndex> {
        self.indices.get(&message).copied()
    }

    /// The validator of `message`, if its rules are compiled.
    pub(crate) fn get(&self, message: MessageIndex) -> Option<&Built<R>> {
        self.index(message).map(|index| self.validator(index).1)
    }

    /// The validator at `index`, along with the message type it validates.
    pub(crate) fn validator(&self, index: ValidatorIndex) -> (MessageIndex, &Built<R>) {
        let (message, built) = &self.validators[index.0 as usize];
        (*message, built)
    }

    /// Every compiled message type, along with its validator.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (MessageIndex, &Built<R>)> {
        self.validators
            .iter()
            .map(|(message, built)| (*message, built))
    }

    /// The index that a validator will receive if `offset` others are added
    /// before it.
    pub(crate) fn next_index(&self, offset: usize) -> ValidatorIndex {
        let index = self.validators.len() + offset;
        ValidatorIndex(u32::try_from(index).expect("fewer than 2^32 message types are validated"))
    }

    /// Adds the validator of `message`, whose rules have not been compiled
    /// before.
    pub(crate) fn push(&mut self, message: MessageIndex, built: Built<R>) -> ValidatorIndex {
        let index = self.next_index(0);
        self.validators.push((message, built));
        self.indices.insert(message, index);
        index
    }
}

/// The message type of a value, whose rules are evaluated after the value's
/// own rules, along with the index of its validator.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Nested {
    pub message: MessageIndex,
    pub validator: ValidatorIndex,
}

/// The rules of one message type. A message-typed field's messages are
/// evaluated inside that field, after its own rules.
pub(crate) struct MessageValidator<R: Runtime> {
    /// The resolved type of this message.
    pub message_type: R::MessageType,
    pub cel: Option<CelPrograms>,
    pub message_oneofs: Vec<MessageOneof<R>>,
    /// The oneofs whose set member is looked up once and is the only member
    /// validated.
    pub oneofs: Vec<OneofValidator<R>>,
    /// The fields validated on their own, or why a field's rules did not
    /// compile. A field whose rules do nothing is left out, and a field
    /// validated through its oneof is in `oneofs`.
    pub fields: Vec<Result<FieldValidator<R>, String>>,
}

/// A `(buf.validate.message).oneof` rule.
pub(crate) struct MessageOneof<R: Runtime> {
    pub fields: Vec<Field<R>>,
    /// The member names joined with `, `, as the violation message prints them.
    pub names: String,
    pub required: bool,
}

/// A oneof whose set member is looked up once and is the only member
/// validated.
pub(crate) struct OneofValidator<R: Runtime> {
    pub oneof: Oneof<R>,
    /// The oneof's path element, if it is `required`.
    pub required: Option<FieldPathElement>,
    /// The members that are validated through the oneof, by field number.
    pub members: Vec<(u32, FieldValidator<R>)>,
}

/// What every field validator needs.
pub(crate) struct FieldHeader<R: Runtime> {
    pub field: Field<R>,
    pub element: FieldPathElement,
    pub required: bool,
    /// Whether to skip the rules when the field is not set.
    pub ignore_empty: bool,
}

/// The rules of one field, shaped by what validating the field needs.
pub(crate) enum FieldValidator<R: Runtime> {
    /// No rule reads the value. The field only has to be set, if it is
    /// required.
    Presence(FieldHeader<R>),
    /// A singular scalar or enum field whose only rules are native checks
    /// of its own kind, as [`scalar_checks`] confirms. Its value is read
    /// directly as its kind, rather than as a [`Val`](crate::protobuf::Val).
    Scalar(FieldHeader<R>, Checks),
    /// A singular message field with no rules of its own, only those of its
    /// message type. An unset field is skipped.
    Message(FieldHeader<R>, Nested),
    /// Any other field, validated through its value, read as a
    /// [`Val`](crate::protobuf::Val), and through the validators of its
    /// items. The value's rules are boxed to keep the other variants small.
    Value(FieldHeader<R>, Box<ValueValidator<R>>, FieldKind<R>),
}

/// Whether `checks` are the native checks for a value of kind `singular`,
/// so that the value can be read as that kind and checked directly.
pub(crate) fn scalar_checks(checks: &Checks, singular: Singular) -> bool {
    matches!(
        (checks, singular),
        (Checks::Bool(_), Singular::Scalar(Scalar::Bool))
            | (
                Checks::Int(_),
                Singular::Scalar(
                    Scalar::Int32
                        | Scalar::Int64
                        | Scalar::Sint32
                        | Scalar::Sint64
                        | Scalar::Sfixed32
                        | Scalar::Sfixed64,
                ),
            )
            | (
                Checks::Uint(_),
                Singular::Scalar(
                    Scalar::Uint32 | Scalar::Uint64 | Scalar::Fixed32 | Scalar::Fixed64
                ),
            )
            | (
                Checks::Double(_),
                Singular::Scalar(Scalar::Float | Scalar::Double)
            )
            | (Checks::String(_), Singular::Scalar(Scalar::String))
            | (Checks::Bytes(_), Singular::Scalar(Scalar::Bytes))
            | (Checks::Enum(_), Singular::Enum)
    )
}

/// How a field holds its values, and the validators of the values inside a
/// container.
pub(crate) enum FieldKind<R: Runtime> {
    Singular,
    List {
        items: Option<Box<ItemValidator<R>>>,
    },
    Map {
        /// The field's path element with the key and value types filled in.
        /// Per-entry violations add the key as its subscript.
        element: FieldPathElement,
        keys: Option<Box<ItemValidator<R>>>,
        values: Option<Box<ItemValidator<R>>>,
    },
}

/// The rules of the elements of a repeated field, or the keys or values of
/// a map.
pub(crate) struct ItemValidator<R: Runtime> {
    /// Whether to skip an item that is its type's zero value.
    pub ignore_empty: bool,
    pub value: ValueValidator<R>,
}

/// The rules of one value.
pub(crate) struct ValueValidator<R: Runtime> {
    /// The `(buf.validate.field).cel` and `.cel_expression` rules.
    pub custom: Option<CelPrograms>,
    /// The native rule checks.
    pub checks: Checks,
    /// The field to validate for wrapper types.
    pub wrapper: Option<Field<R>>,
    /// The fields of a well-known type that its rules read.
    pub wkt: Option<WellKnownFields<R>>,
    /// The predefined CEL of the standard rules for which we do not have
    /// native rule implementations.
    pub predefined: Option<CelPrograms>,
    /// The message type of the value, whose own rules are evaluated after
    /// these.
    pub nested: Option<Nested>,
}

impl<R: Runtime> Default for ValueValidator<R> {
    fn default() -> Self {
        Self {
            custom: None,
            checks: Checks::default(),
            wrapper: None,
            wkt: None,
            predefined: None,
            nested: None,
        }
    }
}

/// The fields of a well-known type that its standard rules read.
pub(crate) enum WellKnownFields<R: Runtime> {
    /// `google.protobuf.Any.type_url`.
    Any { type_url: Field<R> },
    /// `seconds` and `nanos` of `google.protobuf.Duration` or `Timestamp`.
    Time { seconds: Field<R>, nanos: Field<R> },
    /// `google.protobuf.FieldMask.paths`.
    FieldMask { paths: Field<R> },
}

impl<R: Runtime> ValueValidator<R> {
    /// Whether any rule other than those of the nested type reads the
    /// value. The wrapper and well-known type fields are only read by the
    /// checks.
    pub(crate) fn reads_value(&self) -> bool {
        self.custom.is_some() || self.predefined.is_some() || !self.checks.is_empty()
    }

    /// Whether validating a value does nothing.
    pub(crate) fn is_empty(&self) -> bool {
        !self.reads_value() && self.wrapper.is_none() && self.wkt.is_none() && self.nested.is_none()
    }
}

/// The message type of the values of a field with `value` and `kind`,
/// whether the field holds them directly or in its items.
pub(crate) fn nested_type<R: Runtime>(
    value: &ValueValidator<R>,
    kind: &FieldKind<R>,
) -> Option<Nested> {
    let items = match kind {
        FieldKind::List { items } => items.as_deref(),
        FieldKind::Map { values, .. } => values.as_deref(),
        FieldKind::Singular => None,
    };
    value
        .nested
        .or_else(|| items.and_then(|items| items.value.nested))
}

impl<R: Runtime> FieldValidator<R> {
    /// The message type of the field's values, if they are messages.
    pub(crate) fn nested_type(&self) -> Option<Nested> {
        match self {
            Self::Presence(_) | Self::Scalar(..) => None,
            Self::Message(_, nested) => Some(*nested),
            Self::Value(_, value, kind) => nested_type(value, kind),
        }
    }
}

impl<R: Runtime> MessageValidator<R> {
    /// Whether validating a message of this type does nothing.
    pub(crate) fn is_empty(&self) -> bool {
        self.cel.is_none()
            && self.message_oneofs.is_empty()
            && self.oneofs.is_empty()
            && self.fields.is_empty()
    }

    /// Every field validator, whether the field is validated on its own or
    /// through its oneof, or why a field's rules did not compile.
    pub(crate) fn field_validators(
        &self,
    ) -> impl Iterator<Item = Result<&FieldValidator<R>, &String>> {
        let members = self
            .oneofs
            .iter()
            .flat_map(|oneof| oneof.members.iter().map(|(_, field)| Ok(field)));
        self.fields.iter().map(Result::as_ref).chain(members)
    }
}
