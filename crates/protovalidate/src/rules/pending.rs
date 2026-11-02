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

//! A message validator while its rules are being built.
//!
//! A [`PendingValidator`] keeps every field with rules in declaration
//! order, so that the builder can replace a field's rules with an error
//! once the rules of its message type are known, and can prune the message
//! types whose rules do nothing. Finalizing it produces the
//! [`MessageValidator`] that validation uses, which decides how each field
//! is validated.

use std::collections::HashSet;

use buffa_descriptor::MessageIndex;

use super::{
    CelPrograms, FieldHeader, FieldKind, FieldValidator, ItemValidator, MessageOneof,
    MessageValidator, Nested, OneofValidator, ValueValidator, nested_type, scalar_checks,
};
use crate::protobuf::{Field, Kind, Oneof, Runtime, Singular};
use crate::validate::FieldPathElement;

/// The rules of one message type, before they are finalized.
pub(crate) struct PendingValidator<R: Runtime> {
    /// The resolved type of this message.
    pub message_type: R::MessageType,
    pub cel: Option<CelPrograms>,
    pub message_oneofs: Vec<MessageOneof<R>>,
    /// The oneofs that are `required` or have members with rules.
    pub oneofs: Vec<PendingOneof<R>>,
    /// Every field with rules, in declaration order, or why a field's rules
    /// did not compile.
    pub fields: Vec<Result<PendingField<R>, String>>,
}

/// A oneof of a [`PendingValidator`].
pub(crate) struct PendingOneof<R: Runtime> {
    pub oneof: Oneof<R>,
    /// The oneof's path element, if it is `required`.
    pub required: Option<FieldPathElement>,
    /// The positions in [`PendingValidator::fields`] of the members that
    /// the oneof validates, if their rules still compile and do something
    /// once the validator is finalized.
    pub members: Vec<usize>,
}

/// The rules of one field of a [`PendingValidator`].
pub(crate) struct PendingField<R: Runtime> {
    pub field: Field<R>,
    pub element: FieldPathElement,
    pub required: bool,
    /// Whether to skip the rules when the field is not set.
    pub ignore_empty: bool,
    /// The rules of the field's own value. For a container, that is the
    /// whole list or map.
    pub value: ValueValidator<R>,
    pub kind: FieldKind<R>,
}

impl<R: Runtime> PendingField<R> {
    /// The message type of the field's values, if they are messages.
    pub(crate) fn nested_type(&self) -> Option<Nested> {
        nested_type(&self.value, &self.kind)
    }

    /// Whether no rule reads the field's value, meaning it has no rules of
    /// its own and no rules for its items.
    fn value_unused(&self) -> bool {
        self.value.is_empty()
            && match &self.kind {
                FieldKind::Singular => true,
                FieldKind::List { items } => items.is_none(),
                FieldKind::Map { keys, values, .. } => keys.is_none() && values.is_none(),
            }
    }

    /// Whether validating the field does nothing.
    fn does_nothing(&self) -> bool {
        !self.required && self.value_unused()
    }

    /// Shapes the field's validator by what validating the field needs.
    fn finalize(self) -> FieldValidator<R> {
        let unused = self.value_unused();
        let Self {
            field,
            element,
            required,
            ignore_empty,
            value,
            kind,
        } = self;
        let header = FieldHeader {
            field,
            element,
            required,
            ignore_empty,
        };
        if unused {
            return FieldValidator::Presence(header);
        }
        match (header.field.kind(), value) {
            (
                Kind::Singular(singular),
                ValueValidator {
                    custom: None,
                    predefined: None,
                    wrapper: None,
                    wkt: None,
                    nested: None,
                    checks,
                },
            ) if scalar_checks(&checks, singular) => FieldValidator::Scalar(header, checks),
            // An unset message is skipped, which is how an empty value is
            // ignored for a field that tracks presence.
            (
                Kind::Singular(Singular::Message),
                ValueValidator {
                    custom: None,
                    predefined: None,
                    wrapper: None,
                    wkt: None,
                    nested: Some(nested),
                    checks,
                },
            ) if checks.is_empty() && header.ignore_empty && header.field.has_presence() => {
                FieldValidator::Message(header, nested)
            }
            (_, value) => FieldValidator::Value(header, Box::new(value), kind),
        }
    }
}

impl<R: Runtime> PendingValidator<R> {
    /// Whether validating a message of this type does nothing.
    pub(crate) fn is_empty(&self) -> bool {
        self.cel.is_none()
            && self.message_oneofs.is_empty()
            && self.oneofs.iter().all(|oneof| oneof.required.is_none())
            && self
                .fields
                .iter()
                .all(|field| field.as_ref().is_ok_and(PendingField::does_nothing))
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
    ///
    /// A oneof takes the members it validates out of the fields. A member
    /// whose rules did not compile stays a field, so that the error is
    /// reported. A field whose rules do nothing is dropped.
    pub(crate) fn finalize(self) -> MessageValidator<R> {
        let Self {
            message_type,
            cel,
            message_oneofs,
            oneofs,
            fields,
        } = self;
        let mut fields: Vec<Option<Result<PendingField<R>, String>>> =
            fields.into_iter().map(Some).collect();
        let mut validators = Vec::new();
        for oneof in oneofs {
            let mut members = Vec::new();
            for position in oneof.members {
                let member = fields[position]
                    .take_if(|field| field.as_ref().is_ok_and(|field| !field.does_nothing()));
                if let Some(Ok(field)) = member {
                    members.push((field.field.number(), field.finalize()));
                }
            }
            if oneof.required.is_some() || !members.is_empty() {
                validators.push(OneofValidator {
                    oneof: oneof.oneof,
                    required: oneof.required,
                    members,
                });
            }
        }
        let fields = fields
            .into_iter()
            .flatten()
            .filter_map(|field| match field {
                Ok(field) if field.does_nothing() => None,
                Ok(field) => Some(Ok(field.finalize())),
                Err(error) => Some(Err(error)),
            })
            .collect();
        MessageValidator {
            message_type,
            cel,
            message_oneofs,
            oneofs: validators,
            fields,
        }
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
