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
use standard::{
    AnyTest, Check, Cmp, Duration, FieldMaskTest, ListTest, MapTest, ScalarChecks, TimestampTest,
};

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

impl ValidatorIndex {
    /// The index of the validator at `position` in a cache.
    fn at(position: usize) -> Self {
        Self(u32::try_from(position).expect("fewer than 2^32 message types are validated"))
    }

    /// How far this index is past `base`, or `None` if it comes before.
    pub(crate) fn offset_from(self, base: Self) -> Option<usize> {
        self.0.checked_sub(base.0).map(|offset| offset as usize)
    }
}

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

    /// Every validator, along with its index.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (ValidatorIndex, &Built<R>)> {
        self.validators
            .iter()
            .enumerate()
            .map(|(position, (_, built))| (ValidatorIndex::at(position), built))
    }

    /// The index that a validator will receive if `offset` others are added
    /// before it.
    pub(crate) fn next_index(&self, offset: usize) -> ValidatorIndex {
        ValidatorIndex::at(self.validators.len() + offset)
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
    Scalar(FieldHeader<R>, ScalarChecks),
    /// A singular message field with no rules of its own. It is validated
    /// by the rules of its message type, through the validator at this
    /// index. An unset field is skipped.
    Message(FieldHeader<R>, ValidatorIndex),
    /// Any other field, validated through its value, read as a
    /// [`Val`](crate::protobuf::Val), and through the validators of its
    /// items. The value's rules are boxed to keep the other variants small.
    Value(FieldHeader<R>, Box<ValueValidator<R>>, FieldKind<R>),
}

/// Whether `checks` are the native checks for a value of kind `singular`,
/// so that the value can be read as that kind and checked directly.
pub(crate) fn scalar_checks(checks: &ScalarChecks, singular: Singular) -> bool {
    matches!(
        (checks, singular),
        (ScalarChecks::Bool(_), Singular::Scalar(Scalar::Bool))
            | (
                ScalarChecks::Int(_),
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
                ScalarChecks::Uint(_),
                Singular::Scalar(
                    Scalar::Uint32 | Scalar::Uint64 | Scalar::Fixed32 | Scalar::Fixed64
                ),
            )
            | (
                ScalarChecks::Double(_),
                Singular::Scalar(Scalar::Float | Scalar::Double)
            )
            | (ScalarChecks::String(_), Singular::Scalar(Scalar::String))
            | (ScalarChecks::Bytes(_), Singular::Scalar(Scalar::Bytes))
            | (ScalarChecks::Enum(_), Singular::Enum)
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
    /// The native rule checks, with the fields they read.
    pub native: NativeChecks<R>,
    /// The predefined CEL of the standard rules for which we do not have
    /// native rule implementations.
    pub predefined: Option<CelPrograms>,
    /// The index of the validator of the value's message type, whose own
    /// rules are evaluated after these.
    pub nested: Option<ValidatorIndex>,
}

impl<R: Runtime> Default for ValueValidator<R> {
    fn default() -> Self {
        Self {
            custom: None,
            native: NativeChecks::None,
            predefined: None,
            nested: None,
        }
    }
}

/// The native checks of a value, with the fields that evaluating them
/// reads from the value.
pub(crate) enum NativeChecks<R: Runtime> {
    /// Nothing is checked natively.
    None,
    /// Checks of a scalar or enum value.
    Scalar(ScalarChecks),
    /// Checks of the scalar that a wrapper message holds in `field`, which
    /// is how CEL unboxes a wrapper.
    Wrapped {
        checks: ScalarChecks,
        field: Field<R>,
    },
    /// Checks of a repeated field. `element` is the `value` field of
    /// wrapper elements, which `unique` compares the elements through. It
    /// is set only when `unique` is checked on a list of wrappers.
    List {
        checks: Vec<Check<ListTest>>,
        element: Option<Field<R>>,
    },
    /// Checks of a map field.
    Map(Vec<Check<MapTest>>),
    /// Checks of a `google.protobuf.Duration`, read from its `seconds` and
    /// `nanos`.
    Duration {
        checks: Vec<Check<Cmp<Duration>>>,
        seconds: Field<R>,
        nanos: Field<R>,
    },
    /// Checks of a `google.protobuf.Timestamp`, read from its `seconds` and
    /// `nanos`.
    Timestamp {
        checks: Vec<Check<TimestampTest>>,
        seconds: Field<R>,
        nanos: Field<R>,
    },
    /// Checks of a `google.protobuf.FieldMask`, read from its `paths`.
    FieldMask {
        checks: Vec<Check<FieldMaskTest>>,
        paths: Field<R>,
    },
    /// Checks of a `google.protobuf.Any`, read from its `type_url`.
    Any {
        checks: Vec<Check<AnyTest>>,
        type_url: Field<R>,
    },
}

impl<R: Runtime> ValueValidator<R> {
    /// Whether any rule other than those of the nested type reads the
    /// value.
    pub(crate) fn reads_value(&self) -> bool {
        self.custom.is_some()
            || self.predefined.is_some()
            || !matches!(self.native, NativeChecks::None)
    }

    /// Whether validating a value does nothing.
    pub(crate) fn is_empty(&self) -> bool {
        !self.reads_value() && self.nested.is_none()
    }
}

/// The index of the validator of the message type of the values of a field
/// with `value` and `kind`, whether the field holds them directly or in its
/// items.
pub(crate) fn nested_type<R: Runtime>(
    value: &ValueValidator<R>,
    kind: &FieldKind<R>,
) -> Option<ValidatorIndex> {
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
    /// The index of the validator of the message type of the field's
    /// values, if they are messages.
    pub(crate) fn nested_type(&self) -> Option<ValidatorIndex> {
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
