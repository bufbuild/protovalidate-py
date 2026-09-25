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
use crate::validate::{FieldPathElement, FieldRules, Ignore, MessageRules, Rule};

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
#[derive(Clone, Copy)]
struct Target {
    /// The field's kind. An item of a container is singular.
    kind: DescriptorKind,
    /// The value's `FieldDescriptorProto.Type`, for type checks and path
    /// elements.
    ty: Type,
    /// The field's presence.
    presence: FieldPresence,
}

impl Target {
    fn field(field: &FieldDescriptor) -> Self {
        Self {
            kind: field.kind(),
            ty: descriptors::proto_type(field),
            presence: field.presence(),
        }
    }

    /// One list element or map key or value.
    fn item(kind: SingularKind) -> Self {
        Self {
            kind: DescriptorKind::Singular(kind),
            ty: singular_type(kind),
            presence: FieldPresence::Implicit,
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

    /// Whether rules skip an unset value.
    fn ignore_empty(&self, rules: &FieldRules) -> bool {
        rules.ignore == Some(Ignore::IGNORE_IF_ZERO_VALUE)
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

    fn push(
        &mut self,
        id: &str,
        message: &str,
        expression: &str,
        rule_field_number: i32,
        rule_path: Vec<FieldPathElement>,
    ) {
        self.rules.push(CelRule {
            id: id.to_owned(),
            message: message.to_owned(),
            expression: expression.to_owned(),
            rule_field_number,
            rule_path,
        });
    }

    fn push_rule(&mut self, rule: &Rule, rule_field_number: i32, rule_path: Vec<FieldPathElement>) {
        self.push(
            rule.id.as_deref().unwrap_or_default(),
            rule.message.as_deref().unwrap_or_default(),
            rule.expression.as_deref().unwrap_or_default(),
            rule_field_number,
            rule_path,
        );
    }
}

/// The custom rules of a field or message.
fn custom_rules(
    cel_expression: &[String],
    cel: &[Rule],
    expression_path: impl Fn(usize) -> Vec<FieldPathElement>,
    rule_path: impl Fn(usize) -> Vec<FieldPathElement>,
) -> CelExpressions {
    let mut expressions = CelExpressions::new();
    for (index, expression) in cel_expression.iter().enumerate() {
        expressions.push(expression, "", expression, 0, expression_path(index));
    }
    for (index, rule) in cel.iter().enumerate() {
        expressions.push_rule(rule, 0, rule_path(index));
    }
    expressions
}

fn indexed(mut element: FieldPathElement, index: usize) -> FieldPathElement {
    element.subscript = Some(Subscript::Index(index as u64));
    element
}

/// `prefix` followed by `elements`.
fn path(
    prefix: &[FieldPathElement],
    elements: impl IntoIterator<Item = FieldPathElement>,
) -> Vec<FieldPathElement> {
    prefix.iter().cloned().chain(elements).collect()
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

/// The name of the `FieldRules` field holding a scalar type's rules, the
/// field type those rules expect, and the wrapper message type they also
/// accept, if there is one.
fn scalar_rules(rules: &RulesType) -> Option<(&'static str, Type, Option<&'static str>)> {
    Some(match rules {
        RulesType::Float(_) => (
            "float",
            Type::TYPE_FLOAT,
            Some("google.protobuf.FloatValue"),
        ),
        RulesType::Double(_) => (
            "double",
            Type::TYPE_DOUBLE,
            Some("google.protobuf.DoubleValue"),
        ),
        RulesType::Int32(_) => (
            "int32",
            Type::TYPE_INT32,
            Some("google.protobuf.Int32Value"),
        ),
        RulesType::Int64(_) => (
            "int64",
            Type::TYPE_INT64,
            Some("google.protobuf.Int64Value"),
        ),
        RulesType::Uint32(_) => (
            "uint32",
            Type::TYPE_UINT32,
            Some("google.protobuf.UInt32Value"),
        ),
        RulesType::Uint64(_) => (
            "uint64",
            Type::TYPE_UINT64,
            Some("google.protobuf.UInt64Value"),
        ),
        RulesType::Sint32(_) => ("sint32", Type::TYPE_SINT32, None),
        RulesType::Sint64(_) => ("sint64", Type::TYPE_SINT64, None),
        RulesType::Fixed32(_) => ("fixed32", Type::TYPE_FIXED32, None),
        RulesType::Fixed64(_) => ("fixed64", Type::TYPE_FIXED64, None),
        RulesType::Sfixed32(_) => ("sfixed32", Type::TYPE_SFIXED32, None),
        RulesType::Sfixed64(_) => ("sfixed64", Type::TYPE_SFIXED64, None),
        RulesType::Bool(_) => ("bool", Type::TYPE_BOOL, Some("google.protobuf.BoolValue")),
        RulesType::String(_) => (
            "string",
            Type::TYPE_STRING,
            Some("google.protobuf.StringValue"),
        ),
        RulesType::Bytes(_) => (
            "bytes",
            Type::TYPE_BYTES,
            Some("google.protobuf.BytesValue"),
        ),
        RulesType::Enum(_) => ("enum", Type::TYPE_ENUM, None),
        RulesType::Repeated(_)
        | RulesType::Map(_)
        | RulesType::Any(_)
        | RulesType::Duration(_)
        | RulesType::FieldMask(_)
        | RulesType::Timestamp(_) => return None,
    })
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
        let mut failed: HashMap<MessageIndex, String> = HashMap::new();
        failed.extend(
            known
                .iter()
                .filter_map(|(idx, validator)| Some((*idx, validator.as_ref().err()?.clone()))),
        );
        failed.extend(
            built
                .iter()
                .filter_map(|(idx, validator)| Some((*idx, validator.as_ref().err()?.clone()))),
        );
        for (idx, validator) in &mut built {
            let Ok(validator) = validator else {
                continue;
            };
            let message = pool.message(*idx);
            for field in &mut validator.fields {
                let Ok(compiled) = field.as_ref() else {
                    continue;
                };
                let Some(nested) = nested_type(compiled) else {
                    continue;
                };
                let Some(error) = failed.get(&nested) else {
                    continue;
                };
                *field = Err(format!(
                    "failed to compile embedded type {} for {}.{}: {error}",
                    pool.message(nested).full_name(),
                    message.full_name(),
                    compiled.field.name()
                ));
            }
        }
        out.extend(
            built
                .into_iter()
                .map(|(idx, validator)| (idx, validator.map(Arc::new))),
        );
    }

    /// Describes `field` for the runtime, including the resolved type of
    /// the messages it holds.
    fn field(&self, field: &FieldDescriptor) -> Field<R> {
        let message_type = descriptors::message_type(field).map(|idx| {
            self.types
                .get(&idx)
                .unwrap_or_else(|| {
                    panic!(
                        "the message type {} was not resolved before the rules referring to it were built",
                        self.descriptors.pool.message(idx).full_name()
                    )
                })
                .clone()
        });
        Field::from_descriptor(field, message_type)
    }

    fn build_message(&mut self, idx: MessageIndex) -> Result<MessageValidator<R>, String> {
        let pool = &*self.descriptors.pool;
        let message = pool.message(idx);
        let rules = descriptors::message_rules(message);
        let MessageLevel {
            oneofs: message_oneofs,
            members,
        } = match &rules {
            Some(rules) => self.message_oneofs(message, rules)?,
            None => MessageLevel::default(),
        };
        let cel = match &rules {
            Some(rules) => self.compile(custom_rules(
                &rules.cel_expression,
                &rules.cel,
                |_| Vec::new(),
                |_| Vec::new(),
            ))?,
            None => None,
        };

        let mut fields = Vec::new();
        for field in message.fields() {
            let rules = descriptors::field_rules(field);
            // A message-typed field runs its own rules and that of the message type itself.
            if rules.is_none() && descriptors::message_type(field).is_none() {
                continue;
            }
            let mut field_rules = rules.unwrap_or_default();
            // A member of a message oneof is only validated when set, unless
            // it says otherwise.
            if field_rules.ignore.is_none() && members.contains(field.name()) {
                field_rules.ignore = Some(Ignore::IGNORE_IF_ZERO_VALUE);
            }
            if let Some(validator) = self.build_field(field, &field_rules).transpose() {
                fields.push(validator);
            }
        }

        Ok(MessageValidator {
            message_type: self.types[&idx].clone(),
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
        rules: &'m MessageRules,
    ) -> Result<MessageLevel<'m, R>, String> {
        let mut oneofs = Vec::new();
        let mut members = HashSet::new();
        for oneof in &rules.oneof {
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
                        "duplicate \"{name}\" in oneof rule for the message {}",
                        message.full_name()
                    ));
                }
                let Some(field) = message.field_by_name(name) else {
                    return Err(format!(
                        "field \"{name}\" not found in message {}",
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
        field: &FieldDescriptor,
        rules: &FieldRules,
    ) -> Result<Option<FieldValidator<R>>, String> {
        if rules.ignore == Some(Ignore::IGNORE_ALWAYS) {
            return Ok(None);
        }
        let target = Target::field(field);
        let value = self.build_value(target, rules, &[])?;
        let kind = self.build_kind(field, target, rules)?;
        Ok(Some(FieldValidator {
            field: self.field(field),
            element: descriptors::path_element(field),
            required: rules.required.unwrap_or(false),
            ignore_empty: target.ignore_empty(rules),
            value,
            kind,
        }))
    }

    /// The validators of the items inside a container field.
    fn build_kind(
        &mut self,
        field: &FieldDescriptor,
        target: Target,
        rules: &FieldRules,
    ) -> Result<FieldKind<R>, String> {
        let schema = &self.descriptors.schema;
        match target.kind {
            DescriptorKind::Singular(_) => Ok(FieldKind::Singular),
            DescriptorKind::List(element) => {
                let items = match &rules.r#type {
                    Some(RulesType::Repeated(repeated)) => repeated.items.as_option(),
                    _ => None,
                };
                let prefix = [schema.repeated.clone(), schema.repeated_items.clone()];
                Ok(FieldKind::List {
                    items: self.build_item(Target::item(element), items, &prefix)?,
                })
            }
            DescriptorKind::Map { key, value } => {
                let (keys, values) = match &rules.r#type {
                    Some(RulesType::Map(map)) => (map.keys.as_option(), map.values.as_option()),
                    _ => (None, None),
                };
                let keys_prefix = [schema.map.clone(), schema.map_keys.clone()];
                let values_prefix = [schema.map.clone(), schema.map_values.clone()];
                Ok(FieldKind::Map {
                    element: entry_element(field),
                    keys: self.build_item(
                        Target::item(SingularKind::Scalar(key)),
                        keys,
                        &keys_prefix,
                    )?,
                    values: self.build_item(Target::item(value), values, &values_prefix)?,
                })
            }
        }
    }

    /// The validator of a container's items.
    fn build_item(
        &mut self,
        target: Target,
        rules: Option<&FieldRules>,
        prefix: &[FieldPathElement],
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
        Ok(Some(Box::new(ItemValidator {
            ignore_empty: target.ignore_empty(rules),
            value: self.build_value(target, rules, prefix)?,
        })))
    }

    /// The rules of one value. `prefix` is prepended to their rule paths.
    fn build_value(
        &mut self,
        target: Target,
        rules: &FieldRules,
        prefix: &[FieldPathElement],
    ) -> Result<ValueValidator<R>, String> {
        let schema = &self.descriptors.schema;
        let custom = custom_rules(
            &rules.cel_expression,
            &rules.cel,
            |index| path(prefix, [indexed(schema.cel_expression.clone(), index)]),
            |index| path(prefix, [indexed(schema.cel.clone(), index)]),
        );
        let mut value = ValueValidator {
            custom: self.compile(custom)?,
            ..ValueValidator::default()
        };

        let mut predefined = CelExpressions::new();
        if let Some(standard) = &rules.r#type {
            self.standard_rules(target, standard, prefix, &mut value, &mut predefined)?;
        }
        value.predefined = self.compile(predefined)?;

        // A message value's own type is validated after its rules. A
        // container field's messages are its items', validated there.
        value.nested = target.nested();
        Ok(value)
    }

    /// The standard rules set in a `FieldRules`, after checking that they
    /// apply to the value's type.
    fn standard_rules(
        &mut self,
        target: Target,
        standard: &RulesType,
        prefix: &[FieldPathElement],
        value: &mut ValueValidator<R>,
        predefined: &mut CelExpressions,
    ) -> Result<(), String> {
        if let Some((name, expected, wrapper)) = scalar_rules(standard) {
            self.check_scalar_type(&target, expected, wrapper)?;
            value.wrapper = self
                .message(&target)
                .and_then(|wrapper| wrapper.field(1))
                .map(|field| self.field(field));
            let enum_ = match target.kind {
                DescriptorKind::Singular(SingularKind::Enum(idx)) => Some(idx),
                _ => None,
            };
            value.checks = self.standard_checks(name, standard, prefix, enum_, predefined)?;
            return Ok(());
        }
        let name = match standard {
            RulesType::Duration(_) => {
                self.well_known_rules(&target, "duration", descriptors::DURATION)?
            }
            RulesType::FieldMask(_) => {
                self.well_known_rules(&target, "field_mask", descriptors::FIELD_MASK)?
            }
            RulesType::Timestamp(_) => {
                self.well_known_rules(&target, "timestamp", descriptors::TIMESTAMP)?
            }
            RulesType::Any(_) => self.well_known_rules(&target, "any", descriptors::ANY)?,
            RulesType::Repeated(_) => match target.kind {
                DescriptorKind::List(_) => "repeated",
                DescriptorKind::Map { .. } => {
                    return Err("repeated field validator on map field".to_owned());
                }
                DescriptorKind::Singular(_) => {
                    return Err("repeated field validator on non-repeated field".to_owned());
                }
            },
            RulesType::Map(_) => match target.kind {
                DescriptorKind::Map { .. } => "map",
                _ => return Err("map field validator on non-map field".to_owned()),
            },
            _ => return Ok(()),
        };
        value.checks = self.standard_checks(name, standard, prefix, None, predefined)?;
        Ok(())
    }

    /// The message type of a target's values, if they are messages.
    fn message(&self, target: &Target) -> Option<&MessageDescriptor> {
        match target.value() {
            SingularKind::Message(idx) => Some(self.descriptors.pool.message(idx)),
            _ => None,
        }
    }

    /// Checks that the rules in the `FieldRules` field `name`, which apply
    /// only to the well-known type `type_name`, are on a field of that type.
    fn well_known_rules(
        &self,
        target: &Target,
        name: &'static str,
        type_name: &str,
    ) -> Result<&'static str, String> {
        if self.message(target).map(MessageDescriptor::full_name) == Some(type_name) {
            Ok(name)
        } else {
            Err(format!("{name} field validator on non-{name} field"))
        }
    }

    /// Checks that the field has the type the rules apply to, or is that
    /// type's wrapper message.
    fn check_scalar_type(
        &self,
        target: &Target,
        expected: Type,
        wrapper: Option<&str>,
    ) -> Result<(), String> {
        if target.ty == expected {
            return Ok(());
        }
        if target.ty == Type::TYPE_MESSAGE
            && wrapper.is_some()
            && self.message(target).map(MessageDescriptor::full_name) == wrapper
        {
            return Ok(());
        }
        Err(format!(
            "field type does not match rule type: {} != {}",
            descriptors::type_name(target.ty),
            descriptors::type_name(expected)
        ))
    }

    /// The checks of every rule field set in the rules message.
    ///
    /// A rule field with a native check is removed from a copy of the
    /// message as its check is built. Any rule field still set afterwards,
    /// which is an extension or a field without a native check, is
    /// evaluated with its predefined CEL expression instead. That runs
    /// after the native checks, with `rules` bound to the rules message
    /// and `rule` to the field.
    fn standard_checks(
        &self,
        type_field: &'static str,
        standard: &RulesType,
        prefix: &[FieldPathElement],
        enum_: Option<EnumIndex>,
        predefined: &mut CelExpressions,
    ) -> Result<Checks, String> {
        let pool = &self.descriptors.pool;
        let rules = self.dynamic_rules(type_field, standard);
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

        let mut remaining = standard.clone();
        let checks = standard::build::checks(
            type_field,
            &mut remaining,
            &Placement {
                path: &rule_field_path,
                r#enum: enum_,
            },
        )?;

        self.dynamic_rules(type_field, &remaining)
            .for_each_set(&mut |field, _value| {
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
                for rule in &option.cel {
                    predefined.push_rule(rule, number, rule_path(element.clone()));
                }
            });
        predefined.bound = Some((full_name.to_owned(), rules.encode_to_vec()));
        Ok(checks)
    }

    /// The rules message set in `standard`, as a dynamic message.
    fn dynamic_rules(&self, type_field: &str, standard: &RulesType) -> DynamicMessage {
        let pool = &self.descriptors.pool;
        let field_rules = FieldRules {
            r#type: Some(standard.clone()),
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
        match dynamic.get(field) {
            ValueRef::Message(rules) => rules.to_dynamic(),
            _ => unreachable!("FieldRules.{type_field} holds a message"),
        }
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
        let program = self
            .env
            .compile(bound, &to_compile)
            .map_err(|error| error.message().to_owned())?;
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

impl<R: Runtime> Default for MessageLevel<'_, R> {
    fn default() -> Self {
        Self {
            oneofs: Vec::new(),
            members: HashSet::new(),
        }
    }
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
