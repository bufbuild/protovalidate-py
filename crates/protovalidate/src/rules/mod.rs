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
//! [`MessageValidator`]. A message or field whose rules do not compile is
//! the error instead, reported when the walk reaches it. [`eval`] walks a
//! message with the validators, reading it through a
//! [`Runtime`](crate::protobuf::Runtime).

pub(crate) mod build;
pub(crate) mod eval;
pub(crate) mod standard;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use buffa_descriptor::MessageIndex;

use crate::cel;
use crate::protobuf::{Field, Kind, Oneof, Runtime, Singular};
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
    /// The fields with rules, or why a field's rules did not compile.
    pub fields: Vec<Result<FieldValidator<R>, String>>,
    /// The positions in `fields` that are validated on their own, which
    /// excludes fields validated through a oneof and fields whose rules do
    /// nothing. Fields whose rules did not compile are included so that the
    /// error is reported. Set by [`finalize`](Self::finalize).
    pub plain: Box<[u32]>,
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
    /// For each member, the index in `fields` of its validator, if it is
    /// validated through the oneof.
    pub validators: Vec<Option<usize>>,
    /// The field number and position in `fields` of each member that has a
    /// validator, used to find the validator of the set member. Set by
    /// [`MessageValidator::finalize`].
    pub by_number: Box<[(u32, u32)]>,
}

/// How a field is validated, decided once its rules are final.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    /// No rule reads the value. Only whether it is set matters, and only if
    /// it is required.
    Presence,
    /// A singular scalar or enum field whose only rules are native checks.
    /// Its value is read directly as its kind, rather than as a
    /// [`Val`](crate::protobuf::Val).
    Scalar,
    /// A singular message field with no rules of its own, only those of its
    /// message type.
    Message,
    /// Anything else, read as a [`Val`](crate::protobuf::Val).
    Generic,
}

/// The rules of one field.
pub(crate) struct FieldValidator<R: Runtime> {
    pub field: Field<R>,
    pub element: FieldPathElement,
    pub required: bool,
    /// Whether to skip the rules when the field is not set.
    pub ignore_empty: bool,
    /// The rules of the field's own value. For a container, that is the
    /// whole list or map.
    pub value: ValueValidator<R>,
    pub kind: FieldKind<R>,
    /// Set by [`MessageValidator::finalize`].
    pub plan: Plan,
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
    /// Whether any rule other than those of the nested type reads the
    /// value. Set by [`MessageValidator::finalize`].
    pub reads_value: bool,
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
            reads_value: true,
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
    /// Whether validating a value does nothing.
    pub(crate) fn is_empty(&self) -> bool {
        self.custom.is_none()
            && matches!(self.checks, Checks::None)
            && self.wrapper.is_none()
            && self.wkt.is_none()
            && self.predefined.is_none()
            && self.nested.is_none()
    }
}

impl<R: Runtime> FieldValidator<R> {
    /// Whether no rule reads the field's value, meaning it has no rules of
    /// its own and no rules for its items.
    pub(crate) fn value_unused(&self) -> bool {
        self.value.is_empty()
            && match &self.kind {
                FieldKind::Singular => true,
                FieldKind::List { items } => items.is_none(),
                FieldKind::Map { keys, values, .. } => keys.is_none() && values.is_none(),
            }
    }
}

impl<R: Runtime> MessageValidator<R> {
    /// Whether validating a message of this type does nothing.
    pub(crate) fn is_empty(&self) -> bool {
        self.cel.is_none()
            && self.message_oneofs.is_empty()
            && self.oneofs.iter().all(|oneof| oneof.required.is_none())
            && self.fields.iter().all(|field| {
                field
                    .as_ref()
                    .is_ok_and(|field| !field.required && field.value_unused())
            })
    }

    /// Validates field `position` on its own rather than through its oneof,
    /// as is needed when its rules no longer compile.
    pub(crate) fn ungroup(&mut self, position: usize) {
        for oneof in &mut self.oneofs {
            for validator in &mut oneof.validators {
                if *validator == Some(position) {
                    *validator = None;
                }
            }
        }
    }

    /// Stops validating nested messages of the types in `empty`, whose rules
    /// do nothing. Returns whether anything changed.
    pub(crate) fn prune(&mut self, empty: &HashSet<MessageIndex>) -> bool {
        let mut changed = false;
        for field in self.fields.iter_mut().flatten() {
            changed |= prune_value(&mut field.value, empty);
            match &mut field.kind {
                FieldKind::Singular => {}
                FieldKind::List { items } => changed |= prune_items(items, empty),
                FieldKind::Map { keys, values, .. } => {
                    changed |= prune_items(keys, empty);
                    changed |= prune_items(values, empty);
                }
            }
        }
        changed
    }

    /// Decides how each field is validated. Call it once the rules no longer
    /// change, which is after the errors of nested types are propagated and
    /// the nested types whose rules do nothing are pruned.
    pub(crate) fn finalize(&mut self) {
        for field in self.fields.iter_mut().flatten() {
            field.finalize();
        }
        let fields = &self.fields;
        let does_nothing = |position: usize| {
            fields[position]
                .as_ref()
                .is_ok_and(|field| !field.required && field.value_unused())
        };
        for oneof in &mut self.oneofs {
            for validator in &mut oneof.validators {
                if validator.is_some_and(does_nothing) {
                    *validator = None;
                }
            }
            oneof.by_number = oneof
                .oneof
                .members()
                .iter()
                .zip(&oneof.validators)
                .filter_map(|(member, &validator)| Some((member.number(), position(validator?))))
                .collect();
        }
        self.oneofs
            .retain(|oneof| oneof.required.is_some() || !oneof.by_number.is_empty());
        let in_oneof: HashSet<usize> = self
            .oneofs
            .iter()
            .flat_map(|oneof| oneof.validators.iter().flatten().copied())
            .collect();
        self.plain = (0..self.fields.len())
            .filter(|&index| !in_oneof.contains(&index) && !does_nothing(index))
            .map(position)
            .collect();
    }
}

/// Stops validating the nested message of `value` if its type is in
/// `empty`. Returns whether anything changed.
fn prune_value<R: Runtime>(value: &mut ValueValidator<R>, empty: &HashSet<MessageIndex>) -> bool {
    // CEL rules bind the nested message, so it must be kept.
    let prunable = value.custom.is_none()
        && value.predefined.is_none()
        && value
            .nested
            .is_some_and(|nested| empty.contains(&nested.message));
    if prunable {
        value.nested = None;
    }
    prunable
}

/// Prunes the items of a container as [`prune_value`] does. If no rules are
/// left for the items, it removes their validator, so that the elements are
/// not read. Returns whether anything changed.
fn prune_items<R: Runtime>(
    items: &mut Option<Box<ItemValidator<R>>>,
    empty: &HashSet<MessageIndex>,
) -> bool {
    let Some(item) = items else {
        return false;
    };
    if !prune_value(&mut item.value, empty) {
        return false;
    }
    if item.value.is_empty() {
        *items = None;
    }
    true
}

/// Converts a position in a message's fields to the form that compiled rules
/// store.
fn position(index: usize) -> u32 {
    u32::try_from(index).expect("a message has fewer than 2^32 fields")
}

impl<R: Runtime> FieldValidator<R> {
    fn finalize(&mut self) {
        self.value.finalize();
        match &mut self.kind {
            FieldKind::Singular => {}
            FieldKind::List { items } => {
                if let Some(items) = items {
                    items.value.finalize();
                }
            }
            FieldKind::Map { keys, values, .. } => {
                for items in [keys, values].into_iter().flatten() {
                    items.value.finalize();
                }
            }
        }
        let value = &self.value;
        self.plan = match self.field.kind() {
            _ if self.value_unused() => Plan::Presence,
            Kind::Singular(Singular::Scalar(_) | Singular::Enum)
                if value.custom.is_none()
                    && value.predefined.is_none()
                    && value.wrapper.is_none()
                    && value.wkt.is_none() =>
            {
                Plan::Scalar
            }
            // An unset message is skipped, which is how an empty value is
            // ignored for a field that tracks presence.
            Kind::Singular(Singular::Message)
                if !value.reads_value
                    && value.nested.is_some()
                    && self.ignore_empty
                    && self.field.has_presence() =>
            {
                Plan::Message
            }
            _ => Plan::Generic,
        };
    }
}

impl<R: Runtime> ValueValidator<R> {
    fn finalize(&mut self) {
        // The wrapper and well-known type fields are only read by the
        // checks.
        self.reads_value =
            self.custom.is_some() || self.predefined.is_some() || !self.checks.is_empty();
    }
}
