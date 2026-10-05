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

//! Building validators from a message type's `buf.validate` options.
//!
//! Standard rules are checked natively where possible, with custom rules or
//! unknown predefined rules compiled as CEL programs. The structural rules
//! are checked by the validator directly. A message or field whose rules do not
//! compile is kept as the error, reported when validation reaches it.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Write as _};
use std::sync::Arc;

use buffa::editions::FieldPresence;
use buffa_descriptor::generated::descriptor::field_descriptor_proto::Type;
use buffa_descriptor::reflect::{DynamicMessage, ReflectMessage as _, ValueRef};
use buffa_descriptor::{
    EnumIndex, FieldDescriptor, FieldKind as DescriptorKind, MessageDescriptor, MessageIndex,
    SingularKind,
};

use super::standard::build::Placement;
use super::standard::{self, Checks};
use super::{
    CelPrograms, CelRule, FieldKind, FieldValidator, ItemValidator, MessageOneof, MessageValidator,
    OneofRequired, ValidatorCache, ValueValidator,
};
use crate::cel::{Env, Expression};
use crate::descriptors::{self, Descriptors};
use crate::protobuf::{Field, Reader, Runtime};
use crate::validate::__buffa::oneof::field_path_element::Subscript;
use crate::validate::__buffa::oneof::field_rules::Type as RulesType;
use crate::validate::{FieldPathElement, FieldRules, Ignore, MessageOneofRule, Rule};

/// Compiles the rules of message types.
pub(crate) struct Builder<'a, R: Runtime> {
    descriptors: &'a Descriptors,
    env: &'a mut Env,
    /// The resolved type of every message these rules refer to.
    types: HashMap<MessageIndex, R::MessageType>,
}

/// Returns every message type reachable from `root`, including `root`
/// itself, that has no rules in `known` yet.
fn closure<R: Runtime>(
    descriptors: &Descriptors,
    root: MessageIndex,
    known: &ValidatorCache<R>,
) -> HashSet<MessageIndex> {
    let pool = &descriptors.pool;
    let mut closure = HashSet::new();
    let mut pending = vec![root];
    while let Some(idx) = pending.pop() {
        if known.contains_key(&idx) || !closure.insert(idx) {
            continue;
        }
        for field in pool.message(idx).fields() {
            if let Some(nested) = descriptors::message_type(field) {
                pending.push(nested);
            }
        }
    }
    closure
}

/// Resolves every message type that the rules of `root`, or of a type
/// reachable from it, refer to. Types that `known` already has are copied
/// from there; the rest are looked up through `reader`.
///
/// # Errors
///
/// The reader did not know one of the types.
pub(crate) fn resolve<R: Runtime>(
    descriptors: &Descriptors,
    root: MessageIndex,
    known: &ValidatorCache<R>,
    reader: &dyn Reader<R>,
) -> Result<HashMap<MessageIndex, R::MessageType>, R::Error> {
    let pool = &descriptors.pool;
    let mut types = HashMap::new();
    for idx in closure(descriptors, root, known) {
        let referenced = pool
            .message(idx)
            .fields()
            .iter()
            .filter_map(descriptors::message_type);
        for idx in std::iter::once(idx).chain(referenced) {
            if types.contains_key(&idx) {
                continue;
            }
            let message_type = match known.get(&idx) {
                Some(Ok(validator)) => validator.message_type.clone(),
                _ => reader.resolve(pool.message(idx).full_name())?,
            };
            types.insert(idx, message_type);
        }
    }
    Ok(types)
}

/// The value a set of rules applies to.
struct Target<'a> {
    /// The field's kind. An item of a container is singular.
    kind: DescriptorKind,
    /// The value's `FieldDescriptorProto.Type`, for type checks and path
    /// elements.
    ty: Type,
    /// The field's presence.
    presence: FieldPresence,
    /// The value's full name, as compile errors name it.
    name: TargetName<'a>,
}

/// How compile errors name a target: a field by its full name, and a map's
/// keys and values as the `key` and `value` fields of its entry type.
#[derive(Clone, Copy)]
struct TargetName<'a> {
    message: &'a MessageDescriptor,
    field: &'a FieldDescriptor,
    /// For a map's keys or values, `key` or `value`.
    entry: Option<&'static str>,
}

impl Display for TargetName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.", self.message.full_name())?;
        match self.entry {
            None => f.write_str(self.field.name()),
            Some(part) => write!(f, "{}.{part}", MapEntryName(self.field.name())),
        }
    }
}

/// The name of a map field's synthesized entry message: the field name in
/// upper camel case, then `Entry`, as the Protobuf compiler forms it.
struct MapEntryName<'a>(&'a str);

impl Display for MapEntryName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut upper = true;
        for c in self.0.chars() {
            if c == '_' {
                upper = true;
            } else if upper {
                c.to_uppercase().try_for_each(|c| f.write_char(c))?;
                upper = false;
            } else {
                f.write_char(c)?;
            }
        }
        f.write_str("Entry")
    }
}

impl<'a> Target<'a> {
    fn field(message: &'a MessageDescriptor, field: &'a FieldDescriptor) -> Self {
        Self {
            kind: field.kind(),
            ty: descriptors::proto_type(field),
            presence: field.presence(),
            name: TargetName {
                message,
                field,
                entry: None,
            },
        }
    }

    /// One list element or map key or value.
    fn item(kind: SingularKind, name: TargetName<'a>) -> Self {
        Self {
            kind: DescriptorKind::Singular(kind),
            ty: singular_type(kind),
            presence: FieldPresence::Implicit,
            name,
        }
    }

    /// What one value is.
    fn value(&self) -> SingularKind {
        match self.kind {
            DescriptorKind::Singular(kind) | DescriptorKind::List(kind) => kind,
            DescriptorKind::Map { value, .. } => value,
        }
    }

    fn is_singular(&self) -> bool {
        matches!(self.kind, DescriptorKind::Singular(_))
    }

    /// The message type of a singular message field, whose rules are
    /// evaluated after the parent's.
    fn nested(&self) -> Option<MessageIndex> {
        match self.kind {
            DescriptorKind::Singular(SingularKind::Message(nested)) => Some(nested),
            _ => None,
        }
    }

    /// Whether rules with the given `ignore` skip an unset value.
    fn ignore_empty(&self, ignore: Option<Ignore>) -> bool {
        ignore == Some(Ignore::IGNORE_IF_ZERO_VALUE)
            || (self.is_singular() && self.presence != FieldPresence::Implicit)
    }
}

/// The CEL rules of one field or message, before compilation.
struct CelExpressions {
    rules: Vec<CelRule>,
    /// The rules message that `rules` is bound to while predefined rules
    /// run, as its full name and serialized bytes.
    bound: Option<(String, Vec<u8>)>,
}

impl CelExpressions {
    fn new() -> Self {
        Self {
            rules: Vec::new(),
            bound: None,
        }
    }

    fn push(&mut self, rule: Rule, rule_field_number: i32, rule_path: Vec<FieldPathElement>) {
        self.rules.push(CelRule {
            id: rule.id.unwrap_or_default(),
            message: rule.message.unwrap_or_default(),
            expression: rule.expression.unwrap_or_default(),
            rule_field_number,
            rule_path,
        });
    }
}

/// The custom rules of a field or message. A bare expression is its own
/// id, and has no message.
fn custom_rules(
    cel_expression: Vec<String>,
    cel: Vec<Rule>,
    expression_path: impl Fn(usize) -> Vec<FieldPathElement>,
    rule_path: impl Fn(usize) -> Vec<FieldPathElement>,
) -> CelExpressions {
    let mut expressions = CelExpressions::new();
    for (index, expression) in cel_expression.into_iter().enumerate() {
        expressions.rules.push(CelRule {
            id: expression.clone(),
            message: String::new(),
            expression,
            rule_field_number: 0,
            rule_path: expression_path(index),
        });
    }
    for (index, rule) in cel.into_iter().enumerate() {
        expressions.push(rule, 0, rule_path(index));
    }
    expressions
}

fn indexed(mut element: FieldPathElement, index: usize) -> FieldPathElement {
    element.subscript = Some(Subscript::Index(index as u64));
    element
}

/// `prefix` followed by `elements`.
fn path(
    prefix: &[&FieldPathElement],
    elements: impl IntoIterator<Item = FieldPathElement>,
) -> Vec<FieldPathElement> {
    prefix
        .iter()
        .map(|element| (*element).clone())
        .chain(elements)
        .collect()
}

fn singular_type(kind: SingularKind) -> Type {
    match kind {
        SingularKind::Scalar(scalar) => descriptors::scalar_type(scalar),
        SingularKind::Enum(_) => Type::TYPE_ENUM,
        SingularKind::Message(_) => Type::TYPE_MESSAGE,
    }
}

/// The path element for a map field's entries, which carries the key and
/// value types alongside the field.
fn entry_element(field: &FieldDescriptor) -> FieldPathElement {
    let mut element = descriptors::path_element(field);
    if let DescriptorKind::Map { key, value } = field.kind() {
        element.key_type = Some(descriptors::scalar_type(key));
        element.value_type = Some(singular_type(value));
    }
    element
}

/// The message type of a field's values, whether the field holds them
/// directly or in its items.
fn nested_type<R: Runtime>(field: &FieldValidator<R>) -> Option<MessageIndex> {
    let items = match &field.kind {
        FieldKind::List { items } => items.as_deref(),
        FieldKind::Map { values, .. } => values.as_deref(),
        FieldKind::Singular => None,
    };
    field
        .value
        .nested
        .or_else(|| items.and_then(|items| items.value.nested))
}

/// Whether the rules are a scalar type's, which also apply to the type's
/// wrapper message.
fn is_scalar_rules(rules: &RulesType) -> bool {
    !matches!(
        rules,
        RulesType::Repeated(_)
            | RulesType::Map(_)
            | RulesType::Any(_)
            | RulesType::Duration(_)
            | RulesType::FieldMask(_)
            | RulesType::Timestamp(_)
    )
}

/// The `FieldRules` field whose rules apply to values of a well-known
/// message type: its own for `Any`, `Duration`, `FieldMask` and
/// `Timestamp`, and the wrapped scalar's for a wrapper. `None` for any
/// other message type, which takes no standard rules.
fn well_known_rules(type_name: &str) -> Option<&'static str> {
    Some(match type_name {
        descriptors::ANY => "any",
        descriptors::DURATION => "duration",
        descriptors::FIELD_MASK => "field_mask",
        descriptors::TIMESTAMP => "timestamp",
        _ => return wrapper_rules(type_name),
    })
}

/// The `FieldRules` field for the scalar a wrapper message holds, or
/// `None` for a message that is not a wrapper.
fn wrapper_rules(type_name: &str) -> Option<&'static str> {
    Some(match type_name {
        "google.protobuf.BoolValue" => "bool",
        "google.protobuf.BytesValue" => "bytes",
        "google.protobuf.DoubleValue" => "double",
        "google.protobuf.FloatValue" => "float",
        "google.protobuf.Int32Value" => "int32",
        "google.protobuf.Int64Value" => "int64",
        "google.protobuf.StringValue" => "string",
        "google.protobuf.UInt32Value" => "uint32",
        "google.protobuf.UInt64Value" => "uint64",
        _ => return None,
    })
}

/// The name of the `FieldRules` field holding the rules, for messages.
fn rules_name(rules: &RulesType) -> &'static str {
    match rules {
        RulesType::Float(_) => "float",
        RulesType::Double(_) => "double",
        RulesType::Int32(_) => "int32",
        RulesType::Int64(_) => "int64",
        RulesType::Uint32(_) => "uint32",
        RulesType::Uint64(_) => "uint64",
        RulesType::Sint32(_) => "sint32",
        RulesType::Sint64(_) => "sint64",
        RulesType::Fixed32(_) => "fixed32",
        RulesType::Fixed64(_) => "fixed64",
        RulesType::Sfixed32(_) => "sfixed32",
        RulesType::Sfixed64(_) => "sfixed64",
        RulesType::Bool(_) => "bool",
        RulesType::String(_) => "string",
        RulesType::Bytes(_) => "bytes",
        RulesType::Enum(_) => "enum",
        RulesType::Repeated(_) => "repeated",
        RulesType::Map(_) => "map",
        RulesType::Any(_) => "any",
        RulesType::Duration(_) => "duration",
        RulesType::FieldMask(_) => "field_mask",
        RulesType::Timestamp(_) => "timestamp",
    }
}

impl<'a, R: Runtime> Builder<'a, R> {
    /// Creates a builder. `types` holds the resolved type of every message
    /// the rules will refer to.
    pub(crate) fn new(
        descriptors: &'a Descriptors,
        env: &'a mut Env,
        types: HashMap<MessageIndex, R::MessageType>,
    ) -> Self {
        Self {
            descriptors,
            env,
            types,
        }
    }

    /// Compiles the rules of `root` and of every message type reachable
    /// from it, adding them to `out`. Types already in `known` are skipped.
    pub(crate) fn build_closure(
        &mut self,
        root: MessageIndex,
        known: &ValidatorCache<R>,
        out: &mut ValidatorCache<R>,
    ) {
        // The reference is copied out of `self`, so the loop below can
        // still take `&mut self`.
        let descriptors: &'a Descriptors = self.descriptors;
        let pool = &descriptors.pool;
        let mut built: HashMap<MessageIndex, Result<MessageValidator<R>, String>> = HashMap::new();
        for idx in closure(descriptors, root, known) {
            built.insert(idx, self.build_message(idx));
        }

        // A field whose message type did not compile does not compile either.
        let mut failures = Vec::new();
        for (&idx, validator) in &built {
            let Ok(validator) = validator else {
                continue;
            };
            for (position, field) in validator.fields.iter().enumerate() {
                let Ok(compiled) = field else {
                    continue;
                };
                let Some(nested) = nested_type(compiled) else {
                    continue;
                };
                let error = match known.get(&nested) {
                    Some(validator) => validator.as_ref().err(),
                    None => built
                        .get(&nested)
                        .and_then(|validator| validator.as_ref().err()),
                };
                let Some(error) = error else {
                    continue;
                };
                failures.push((
                    idx,
                    position,
                    format!(
                        "failed to compile embedded type {} for {}.{}: {error}",
                        pool.message(nested).full_name(),
                        pool.message(idx).full_name(),
                        compiled.field.name()
                    ),
                ));
            }
        }
        for (idx, position, error) in failures {
            if let Some(Ok(validator)) = built.get_mut(&idx) {
                validator.fields[position] = Err(error);
            }
        }
        out.extend(
            built
                .into_iter()
                .map(|(idx, validator)| (idx, validator.map(Arc::new))),
        );
    }

    /// The resolved type of a message, which [`resolve`] found before any
    /// rules were built.
    fn resolved(&self, idx: MessageIndex) -> R::MessageType {
        self.types
            .get(&idx)
            .unwrap_or_else(|| {
                panic!(
                    "the message type {} was not resolved before the rules referring to it were built",
                    self.descriptors.pool.message(idx).full_name()
                )
            })
            .clone()
    }

    /// Describes `field` for the runtime, including the resolved type of
    /// the messages it holds.
    fn field(&self, field: &FieldDescriptor) -> Field<R> {
        let message_type = descriptors::message_type(field).map(|idx| self.resolved(idx));
        Field::from_descriptor(field, message_type)
    }

    fn build_message(&mut self, idx: MessageIndex) -> Result<MessageValidator<R>, String> {
        let pool = &*self.descriptors.pool;
        let message = pool.message(idx);
        let (custom, oneof_rules) = match descriptors::message_rules(message) {
            Some(rules) => (
                Some(custom_rules(
                    rules.cel_expression,
                    rules.cel,
                    |_| Vec::new(),
                    |_| Vec::new(),
                )),
                rules.oneof,
            ),
            None => (None, Vec::new()),
        };
        let MessageLevel {
            oneofs: message_oneofs,
            members,
        } = self.message_oneofs(message, &oneof_rules)?;
        let cel = match custom {
            Some(custom) => self.compile(custom)?,
            None => None,
        };

        let mut fields = Vec::new();
        for field in message.fields() {
            let rules = descriptors::field_rules(field);
            // A message-typed field runs its own rules and that of the message type itself.
            if rules.is_none() && descriptors::message_type(field).is_none() {
                continue;
            }
            let mut rules = rules.unwrap_or_default();
            // A member of a message oneof is only validated when set, unless
            // it says otherwise.
            if rules.ignore.is_none() && members.contains(field.name()) {
                rules.ignore = Some(Ignore::IGNORE_IF_ZERO_VALUE);
            }
            if let Some(validator) = self.build_field(message, field, rules).transpose() {
                fields.push(validator);
            }
        }

        Ok(MessageValidator {
            message_type: self.resolved(idx),
            cel,
            message_oneofs,
            oneofs: self.required_oneofs(message),
            fields,
        })
    }

    /// The oneof declarations marked `required`.
    fn required_oneofs(&self, message: &MessageDescriptor) -> Vec<OneofRequired<R>> {
        message
            .oneofs()
            .iter()
            .filter(|oneof| {
                descriptors::oneof_rules(oneof).is_some_and(|rules| rules.required.unwrap_or(false))
            })
            .map(|oneof| OneofRequired {
                element: descriptors::oneof_path_element(oneof.name()),
                members: oneof
                    .field_indices()
                    .iter()
                    .filter_map(|&index| message.fields().get(usize::from(index)))
                    .map(|field| self.field(field))
                    .collect(),
            })
            .collect()
    }

    /// The `(buf.validate.message).oneof` rules, and the fields they name.
    fn message_oneofs<'m>(
        &self,
        message: &'m MessageDescriptor,
        rules: &'m [MessageOneofRule],
    ) -> Result<MessageLevel<'m, R>, String> {
        let mut oneofs = Vec::new();
        let mut members = HashSet::new();
        for oneof in rules {
            if oneof.fields.is_empty() {
                return Err(format!(
                    "at least one field must be specified in oneof rule for the message {}",
                    message.full_name()
                ));
            }
            let mut seen = HashSet::new();
            let mut fields = Vec::with_capacity(oneof.fields.len());
            for name in &oneof.fields {
                if !seen.insert(name.as_str()) {
                    return Err(format!(
                        "duplicate {name} in oneof rule for the message {}",
                        message.full_name()
                    ));
                }
                let Some(field) = message.field_by_name(name) else {
                    return Err(format!(
                        "field {name} not found in message {}",
                        message.full_name()
                    ));
                };
                fields.push(self.field(field));
            }
            oneofs.push(MessageOneof {
                fields,
                names: oneof.fields.join(", "),
                required: oneof.required.unwrap_or(false),
            });
            members.extend(oneof.fields.iter().map(String::as_str));
        }
        Ok(MessageLevel { oneofs, members })
    }

    /// The validator of a field, or `None` when its rules say to always
    /// ignore it.
    fn build_field(
        &mut self,
        message: &MessageDescriptor,
        field: &FieldDescriptor,
        rules: FieldRules,
    ) -> Result<Option<FieldValidator<R>>, String> {
        if rules.ignore == Some(Ignore::IGNORE_ALWAYS) {
            return Ok(None);
        }
        let target = Target::field(message, field);
        let required = rules.required.unwrap_or(false);
        let ignore_empty = target.ignore_empty(rules.ignore);
        let (value, remaining) = self.build_value(&target, rules, &[])?;
        let kind = self.build_kind(field, &target, remaining)?;
        Ok(Some(FieldValidator {
            field: self.field(field),
            element: descriptors::path_element(field),
            required,
            ignore_empty,
            value,
            kind,
        }))
    }

    /// The validators of the items inside a container field. `remaining`
    /// is what is left of the field's standard rules once its own checks
    /// were taken from them, which for a container is the rules of its
    /// items.
    fn build_kind(
        &mut self,
        field: &FieldDescriptor,
        target: &Target<'_>,
        remaining: Option<RulesType>,
    ) -> Result<FieldKind<R>, String> {
        let schema = &self.descriptors.schema;
        match target.kind {
            DescriptorKind::Singular(_) => Ok(FieldKind::Singular),
            DescriptorKind::List(element) => {
                let items = match remaining {
                    Some(RulesType::Repeated(mut repeated)) => repeated.items.take(),
                    _ => None,
                };
                let prefix = [&schema.repeated, &schema.repeated_items];
                let item = Target::item(element, target.name);
                Ok(FieldKind::List {
                    items: self.build_item(&item, items, &prefix).map_err(|error| {
                        format!(
                            "failed to compile items rules for repeated {}: {error}",
                            target.name
                        )
                    })?,
                })
            }
            DescriptorKind::Map { key, value } => {
                let (keys, values) = match remaining {
                    Some(RulesType::Map(mut map)) => (map.keys.take(), map.values.take()),
                    _ => (None, None),
                };
                let keys_prefix = [&schema.map, &schema.map_keys];
                let values_prefix = [&schema.map, &schema.map_values];
                let entry = |part| TargetName {
                    entry: Some(part),
                    ..target.name
                };
                let key_target = Target::item(SingularKind::Scalar(key), entry("key"));
                let value_target = Target::item(value, entry("value"));
                Ok(FieldKind::Map {
                    element: entry_element(field),
                    keys: self
                        .build_item(&key_target, keys, &keys_prefix)
                        .map_err(|error| {
                            format!(
                                "failed to compile key rules for map {}: {error}",
                                target.name
                            )
                        })?,
                    values: self
                        .build_item(&value_target, values, &values_prefix)
                        .map_err(|error| {
                            format!(
                                "failed to compile value rules for map {}: {error}",
                                target.name
                            )
                        })?,
                })
            }
        }
    }

    /// The validator of a container's items.
    fn build_item(
        &mut self,
        target: &Target<'_>,
        rules: Option<FieldRules>,
        prefix: &[&FieldPathElement],
    ) -> Result<Option<Box<ItemValidator<R>>>, String> {
        let Some(rules) = rules else {
            return Ok(target.nested().map(|nested| {
                Box::new(ItemValidator {
                    ignore_empty: false,
                    value: ValueValidator {
                        nested: Some(nested),
                        ..ValueValidator::default()
                    },
                })
            }));
        };
        if rules.ignore == Some(Ignore::IGNORE_ALWAYS) {
            return Ok(None);
        }
        let ignore_empty = target.ignore_empty(rules.ignore);
        // An item is not a container, so nothing of its standard rules
        // remains once its checks are taken.
        let (value, _) = self.build_value(target, rules, prefix)?;
        Ok(Some(Box::new(ItemValidator {
            ignore_empty,
            value,
        })))
    }

    /// The rules of one value. `prefix` is prepended to their rule paths.
    /// Also returns what is left of the standard rules message once the
    /// native checks took their fields from it; see [`Self::build_kind`].
    fn build_value(
        &mut self,
        target: &Target<'_>,
        rules: FieldRules,
        prefix: &[&FieldPathElement],
    ) -> Result<(ValueValidator<R>, Option<RulesType>), String> {
        let schema = &self.descriptors.schema;
        let custom = custom_rules(
            rules.cel_expression,
            rules.cel,
            |index| path(prefix, [indexed(schema.cel_expression.clone(), index)]),
            |index| path(prefix, [indexed(schema.cel.clone(), index)]),
        );
        let mut value = ValueValidator {
            custom: self.compile(custom)?,
            ..ValueValidator::default()
        };

        let mut predefined = CelExpressions::new();
        let remaining = match rules.r#type {
            Some(standard) => {
                Some(self.standard_rules(target, standard, prefix, &mut value, &mut predefined)?)
            }
            None => None,
        };
        value.predefined = self.compile(predefined)?;

        // A message value's own type is validated after its rules. A
        // container field's messages are its items', validated there.
        value.nested = target.nested();
        Ok((value, remaining))
    }

    /// The standard rules set in a `FieldRules`, after checking that they
    /// are the rules for the value's type. Returns what is left of them
    /// once the native checks took their fields.
    fn standard_rules(
        &mut self,
        target: &Target<'_>,
        standard: RulesType,
        prefix: &[&FieldPathElement],
        value: &mut ValueValidator<R>,
        predefined: &mut CelExpressions,
    ) -> Result<RulesType, String> {
        let got = rules_name(&standard);
        match self.expected_rules(target) {
            Some(expected) if expected == got => {}
            Some(expected) => {
                return Err(format!(
                    "expected rule \"buf.validate.FieldRules.{expected}\", \
                     got \"buf.validate.FieldRules.{got}\" on field \"{}\"",
                    target.name
                ));
            }
            None => {
                return Err(format!(
                    "mismatched message rules, \"buf.validate.FieldRules.{got}\" \
                     is not a valid rule for field \"{}\"",
                    target.name
                ));
            }
        }
        let enum_ = if is_scalar_rules(&standard) {
            value.wrapper = self
                .message(target)
                .and_then(|wrapper| wrapper.field(1))
                .map(|field| self.field(field));
            match target.kind {
                DescriptorKind::Singular(SingularKind::Enum(idx)) => Some(idx),
                _ => None,
            }
        } else {
            if let (RulesType::Repeated(rules), DescriptorKind::List(SingularKind::Message(_))) =
                (&standard, target.kind)
                && rules.unique == Some(true)
            {
                // `unique()` compares scalars. Wrapper elements stand for
                // the scalars they hold, as they do for every other rule,
                // and are compared through their `value` field. Any other
                // message element would fail every validation.
                let wrapper = self
                    .message(target)
                    .filter(|message| wrapper_rules(message.full_name()).is_some())
                    .and_then(|wrapper| wrapper.field(1));
                let Some(wrapper) = wrapper else {
                    return Err("repeated.unique is not supported for message items".to_owned());
                };
                value.wrapper = Some(self.field(wrapper));
            }
            None
        };
        let (checks, remaining) = self.standard_checks(got, standard, prefix, enum_, predefined)?;
        value.checks = checks;
        Ok(remaining)
    }

    /// The message type of a target's values, if they are messages.
    fn message(&self, target: &Target<'_>) -> Option<&MessageDescriptor> {
        match target.value() {
            SingularKind::Message(idx) => Some(self.descriptors.pool.message(idx)),
            _ => None,
        }
    }

    /// The `FieldRules` field whose rules apply to the target: `repeated`
    /// and `map` for the containers, whose elements' rules go under
    /// `items`, `keys` and `values`; the scalar type's for a scalar, an
    /// enum or a wrapper; the well-known type's for the others that have
    /// one. `None` for a message type that takes no standard rules.
    fn expected_rules(&self, target: &Target<'_>) -> Option<&'static str> {
        match target.kind {
            DescriptorKind::List(_) => Some("repeated"),
            DescriptorKind::Map { .. } => Some("map"),
            DescriptorKind::Singular(SingularKind::Message(idx)) => {
                well_known_rules(self.descriptors.pool.message(idx).full_name())
            }
            DescriptorKind::Singular(_) => Some(descriptors::type_name(target.ty)),
        }
    }

    /// The checks of every rule field set in the rules message.
    ///
    /// A rule field with a native check is removed from the message as its
    /// check is built. Any rule field still set afterwards, which is an
    /// extension or a field without a native check, is evaluated with its
    /// predefined CEL expression instead. That runs after the native
    /// checks, with `rules` bound to the rules message and `rule` to the
    /// field. What remains of the message is returned with the checks.
    fn standard_checks(
        &self,
        type_field: &'static str,
        standard: RulesType,
        prefix: &[&FieldPathElement],
        enum_: Option<EnumIndex>,
        predefined: &mut CelExpressions,
    ) -> Result<(Checks, RulesType), String> {
        let pool = &self.descriptors.pool;
        let (rules, mut remaining) = self.dynamic_rules(type_field, standard);
        let idx = rules.message_index();
        let message = rules.message_descriptor();
        let full_name = message.full_name();
        if !rules.unknown_fields().is_empty() {
            return Err(format!("unknown rules in {full_name}"));
        }
        let type_element = self.descriptors.schema.type_element(pool, type_field);
        let rule_path = |element: FieldPathElement| path(prefix, [type_element.clone(), element]);
        let rule_field_path = |number: u32| {
            let field = message
                .field(number)
                .unwrap_or_else(|| panic!("{full_name} has a field {number}"));
            rule_path(descriptors::path_element(field))
        };

        let checks = standard::build::checks(
            type_field,
            &mut remaining,
            &Placement {
                path: &rule_field_path,
                r#enum: enum_,
            },
        )?;

        let (unchecked, remaining) = self.dynamic_rules(type_field, remaining);
        unchecked.for_each_set(&mut |field, _value| {
            // `example` documents the field; its rule is `true`.
            if field.name() == "example" {
                return;
            }
            let element = match pool.extension_for(idx, field.number()) {
                Some(extension) if message.field(field.number()).is_none() => {
                    descriptors::extension_path_element(extension)
                }
                _ => descriptors::path_element(field),
            };
            let Some(option) = descriptors::predefined_rules(field) else {
                return;
            };
            let number = descriptors::field_number(field.number());
            for rule in option.cel {
                predefined.push(rule, number, rule_path(element.clone()));
            }
        });
        predefined.bound = Some((full_name.to_owned(), rules.encode_to_vec()));
        Ok((checks, remaining))
    }

    /// The rules message set in `standard`, as a dynamic message, with
    /// `standard` handed back.
    fn dynamic_rules(&self, type_field: &str, standard: RulesType) -> (DynamicMessage, RulesType) {
        let pool = &self.descriptors.pool;
        let field_rules = FieldRules {
            r#type: Some(standard),
            ..Default::default()
        };
        let dynamic = DynamicMessage::from_message(
            &field_rules,
            Arc::clone(pool),
            self.descriptors.schema.field_rules,
        );
        let field = dynamic
            .message_descriptor()
            .field_by_name(type_field)
            .unwrap_or_else(|| panic!("FieldRules.{type_field} exists"));
        let rules = match dynamic.get(field) {
            ValueRef::Message(rules) => rules.to_dynamic(),
            _ => unreachable!("FieldRules.{type_field} holds a message"),
        };
        let Some(standard) = field_rules.r#type else {
            unreachable!("the rules were just put in FieldRules.{type_field}")
        };
        (rules, standard)
    }

    fn compile(&mut self, expressions: CelExpressions) -> Result<Option<CelPrograms>, String> {
        if expressions.rules.is_empty() {
            return Ok(None);
        }
        let bound = expressions
            .bound
            .as_ref()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()));
        let to_compile: Vec<Expression<'_>> = expressions
            .rules
            .iter()
            .map(|rule| Expression {
                expression: &rule.expression,
                rule_field_number: rule.rule_field_number,
            })
            .collect();
        let program = match self.env.compile(bound, &to_compile) {
            Ok(program) => program,
            Err(error) => {
                // The engine compiles the expressions together and names
                // none in its error, so find the one at fault.
                let culprit =
                    expressions
                        .rules
                        .iter()
                        .zip(&to_compile)
                        .find_map(|(rule, expression)| {
                            self.env
                                .compile(bound, std::slice::from_ref(expression))
                                .err()
                                .map(|error| (rule.id.as_str(), error))
                        });
                return Err(match culprit {
                    Some((id, error)) => {
                        format!("failed to compile expression {id}: {}", error.message())
                    }
                    None => error.message().to_owned(),
                });
            }
        };
        Ok(Some(CelPrograms {
            program,
            rules: expressions.rules,
        }))
    }
}

/// A message's oneof rules, and the fields they name.
struct MessageLevel<'m, R: Runtime> {
    oneofs: Vec<MessageOneof<R>>,
    members: HashSet<&'m str>,
}

#[cfg(all(test, feature = "cel"))]
mod tests {
    use buffa::{ExtensionSet as _, UnknownField, UnknownFieldData};
    use buffa_descriptor::generated::descriptor::field_descriptor_proto::{Label, Type};
    use buffa_descriptor::generated::descriptor::{
        DescriptorProto, FieldDescriptorProto, FieldOptions, FileDescriptorProto, FileDescriptorSet,
    };

    use super::Builder;
    use crate::cel::{ScalarValue, This, Value};
    use crate::descriptors::{self, Descriptors};
    use crate::protobuf::testing::Untyped;
    use crate::rules::ValidatorCache;
    use crate::rules::standard::Checks;
    use crate::validate::__buffa::oneof::field_rules::Type as RulesType;
    use crate::validate::{FIELD, FieldRules, PREDEFINED, PredefinedRules, Rule, StringRules};
    use crate::validator::cel_env;

    fn schema_with_test_len() -> FileDescriptorSet {
        let mut set = descriptors::base_file_set();
        let validate = set
            .file
            .iter_mut()
            .find(|file| file.name.as_deref() == Some("buf/validate/validate.proto"))
            .expect("validate.proto is in the embedded set");
        let string_rules = validate
            .message_type
            .iter_mut()
            .find(|message| message.name.as_deref() == Some("StringRules"))
            .expect("StringRules is in validate.proto");
        let mut options = FieldOptions::default();
        options.set_extension(
            &PREDEFINED,
            PredefinedRules {
                cel: vec![Rule {
                    id: Some("string.test_len".to_owned()),
                    message: Some("must be test_len long".to_owned()),
                    expression: Some("size(this) == rule".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        string_rules.field.push(FieldDescriptorProto {
            name: Some("test_len".to_owned()),
            number: Some(999),
            label: Some(Label::LABEL_OPTIONAL),
            r#type: Some(Type::TYPE_UINT64),
            options: Some(options).into(),
            ..Default::default()
        });

        let mut string = StringRules::default();
        string.unknown_fields_mut().push(UnknownField {
            number: 999,
            data: UnknownFieldData::Varint(3),
        });
        let mut options = FieldOptions::default();
        options.set_extension(
            &FIELD,
            FieldRules {
                r#type: Some(RulesType::String(Box::new(string))),
                ..Default::default()
            },
        );
        set.file.push(FileDescriptorProto {
            name: Some("test.proto".to_owned()),
            package: Some("test".to_owned()),
            dependency: vec!["buf/validate/validate.proto".to_owned()],
            message_type: vec![DescriptorProto {
                name: Some("Message".to_owned()),
                field: vec![FieldDescriptorProto {
                    name: Some("s".to_owned()),
                    number: Some(1),
                    label: Some(Label::LABEL_OPTIONAL),
                    r#type: Some(Type::TYPE_STRING),
                    options: Some(options).into(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            syntax: Some("proto3".to_owned()),
            ..Default::default()
        });
        set
    }

    #[test]
    fn unknown_standard_rule_falls_back_to_predefined_cel() {
        let set = schema_with_test_len();
        let mut env = cel_env(&set);
        let descriptors = Descriptors::from_set(set);
        let index = descriptors
            .pool
            .message_index("test.Message")
            .expect("test.Message is in the pool");
        let known = ValidatorCache::<Untyped>::default();
        let mut out = ValidatorCache::default();
        let types =
            super::resolve(&descriptors, index, &known, &Untyped).expect("every type resolves");
        Builder::new(&descriptors, &mut env, types).build_closure(index, &known, &mut out);

        let validator = out[&index].as_ref().expect("the message compiles");
        let field = validator.fields[0].as_ref().expect("the field compiles");
        assert!(
            matches!(&field.value.checks, Checks::String(checks) if checks.is_empty()),
            "no native check covers test_len"
        );
        let predefined = field
            .value
            .predefined
            .as_ref()
            .expect("test_len is checked by its predefined CEL");
        assert_eq!(predefined.rules.len(), 1);
        assert_eq!(predefined.rules[0].id, "string.test_len");
        assert_eq!(predefined.rules[0].message, "must be test_len long");
        let rule_path: Vec<&str> = predefined.rules[0]
            .rule_path
            .iter()
            .filter_map(|element| element.field_name.as_deref())
            .collect();
        assert_eq!(rule_path, ["string", "test_len"]);
        // `rule` is bound to the rule's value, 3.
        assert!(matches!(
            predefined
                .program
                .eval(0, This::Scalar(ScalarValue::String("abc"))),
            Ok(Value::Bool(true))
        ));
        assert!(matches!(
            predefined
                .program
                .eval(0, This::Scalar(ScalarValue::String("ab"))),
            Ok(Value::Bool(false))
        ));
    }
}
