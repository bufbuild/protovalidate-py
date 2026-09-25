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

use std::collections::HashMap;
use std::sync::Arc;

use buffa_descriptor::MessageIndex;

use crate::cel;
use crate::protobuf::{Field, Runtime};
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

/// The compiled rules of every message type validated so far.
pub(crate) type ValidatorCache<R> = HashMap<MessageIndex, Built<R>, foldhash::fast::RandomState>;

/// The rules of one message type. A message-typed field's messages are
/// evaluated inside that field, after its own rules.
pub(crate) struct MessageValidator<R: Runtime> {
    /// The resolved type of this message.
    pub message_type: R::MessageType,
    pub cel: Option<CelPrograms>,
    pub message_oneofs: Vec<MessageOneof<R>>,
    pub oneofs: Vec<OneofRequired<R>>,
    /// The fields with rules, or why a field's rules did not compile.
    pub fields: Vec<Result<FieldValidator<R>, String>>,
}

/// A `(buf.validate.message).oneof` rule.
pub(crate) struct MessageOneof<R: Runtime> {
    pub fields: Vec<Field<R>>,
    /// The member names joined with `, `, as the violation message prints them.
    pub names: String,
    pub required: bool,
}

/// A `(buf.validate.oneof).required` rule.
pub(crate) struct OneofRequired<R: Runtime> {
    pub element: FieldPathElement,
    pub members: Vec<Field<R>>,
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
    /// The predefined CEL of the standard rules for which we do not have
    /// native rule implementations.
    pub predefined: Option<CelPrograms>,
    /// The message type of the value, whose own rules are evaluated after
    /// these.
    pub nested: Option<MessageIndex>,
}

impl<R: Runtime> Default for ValueValidator<R> {
    fn default() -> Self {
        Self {
            custom: None,
            checks: Checks::default(),
            wrapper: None,
            predefined: None,
            nested: None,
        }
    }
}
