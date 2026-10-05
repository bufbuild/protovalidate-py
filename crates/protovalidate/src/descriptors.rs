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

//! The descriptor pool the validator reflects over, and the pieces of the
//! `buf.validate` schema it reads from that pool.

use std::sync::Arc;

use buffa::ExtensionSet as _;
use buffa_descriptor::generated::descriptor::FileDescriptorSet;
use buffa_descriptor::generated::descriptor::field_descriptor_proto::Type;
use buffa_descriptor::{
    DescriptorPool, ExtensionDescriptor, FieldDescriptor, FieldKind, MessageDescriptor,
    MessageIndex, ScalarType, SingularKind,
};

use crate::validate::{FieldPathElement, FieldRules, MessageRules, OneofRules, PredefinedRules};

/// `buf/validate/validate.proto` and its imports, as a serialized
/// `FileDescriptorSet`. Built by `poe generate`.
pub(crate) const BASE_DESCRIPTOR_SET: &[u8] = include_bytes!("gen/buf.validate.binpb");

pub(crate) const FIELD_RULES: &str = "buf.validate.FieldRules";
pub(crate) const ANY: &str = "google.protobuf.Any";
pub(crate) const DURATION: &str = "google.protobuf.Duration";
pub(crate) const TIMESTAMP: &str = "google.protobuf.Timestamp";
pub(crate) const FIELD_MASK: &str = "google.protobuf.FieldMask";

/// Decodes a serialized descriptor message, such as a `FileDescriptorSet`
/// or a `FileDescriptorProto`.
pub(crate) fn decode<M: buffa::Message + buffa::MessageName>(bytes: &[u8]) -> Result<M, String> {
    // Descriptors are trusted schema, and a large set exceeds the
    // element-memory bound buffa applies to untrusted input.
    buffa::DecodeOptions::new()
        .with_element_memory_limit(usize::MAX)
        .decode_from_slice(bytes)
        .map_err(|error| format!("could not parse {}: {error}", M::NAME))
}

/// The `buf.validate` schema, decoded.
pub(crate) fn base_file_set() -> FileDescriptorSet {
    decode(BASE_DESCRIPTOR_SET).expect("embedded descriptor set decodes")
}

/// The `FieldDescriptorProto.Type` of a field, as protovalidate reports it
/// in field paths.
pub(crate) fn proto_type(field: &FieldDescriptor) -> Type {
    let singular = match field.kind() {
        FieldKind::Singular(kind) | FieldKind::List(kind) => kind,
        FieldKind::Map { .. } => return Type::TYPE_MESSAGE,
    };
    match singular {
        SingularKind::Scalar(scalar) => scalar_type(scalar),
        SingularKind::Enum(_) => Type::TYPE_ENUM,
        SingularKind::Message(_) if field.is_delimited() => Type::TYPE_GROUP,
        SingularKind::Message(_) => Type::TYPE_MESSAGE,
    }
}

pub(crate) fn scalar_type(scalar: ScalarType) -> Type {
    match scalar {
        ScalarType::Double => Type::TYPE_DOUBLE,
        ScalarType::Float => Type::TYPE_FLOAT,
        ScalarType::Int64 => Type::TYPE_INT64,
        ScalarType::Uint64 => Type::TYPE_UINT64,
        ScalarType::Int32 => Type::TYPE_INT32,
        ScalarType::Fixed64 => Type::TYPE_FIXED64,
        ScalarType::Fixed32 => Type::TYPE_FIXED32,
        ScalarType::Bool => Type::TYPE_BOOL,
        ScalarType::String => Type::TYPE_STRING,
        ScalarType::Bytes => Type::TYPE_BYTES,
        ScalarType::Uint32 => Type::TYPE_UINT32,
        ScalarType::Sfixed32 => Type::TYPE_SFIXED32,
        ScalarType::Sfixed64 => Type::TYPE_SFIXED64,
        ScalarType::Sint32 => Type::TYPE_SINT32,
        ScalarType::Sint64 => Type::TYPE_SINT64,
    }
}

/// The name protovalidate prints for a type, as in `field type does not match
/// rule type: string != int32`.
pub(crate) fn type_name(ty: Type) -> &'static str {
    match ty {
        Type::TYPE_DOUBLE => "double",
        Type::TYPE_FLOAT => "float",
        Type::TYPE_INT64 => "int64",
        Type::TYPE_UINT64 => "uint64",
        Type::TYPE_INT32 => "int32",
        Type::TYPE_FIXED64 => "fixed64",
        Type::TYPE_FIXED32 => "fixed32",
        Type::TYPE_BOOL => "bool",
        Type::TYPE_STRING => "string",
        Type::TYPE_GROUP => "group",
        Type::TYPE_MESSAGE => "message",
        Type::TYPE_BYTES => "bytes",
        Type::TYPE_UINT32 => "uint32",
        Type::TYPE_ENUM => "enum",
        Type::TYPE_SFIXED32 => "sfixed32",
        Type::TYPE_SFIXED64 => "sfixed64",
        Type::TYPE_SINT32 => "sint32",
        Type::TYPE_SINT64 => "sint64",
    }
}

/// The path element naming `field`.
pub(crate) fn path_element(field: &FieldDescriptor) -> FieldPathElement {
    path_element_named(field, field.name().to_owned())
}

/// The path element naming an extension field, which protovalidate renders
/// as `[pkg.name]`.
pub(crate) fn extension_path_element(extension: &ExtensionDescriptor) -> FieldPathElement {
    path_element_named(extension.field(), format!("[{}]", extension.full_name()))
}

/// A field number as `FieldPathElement` and CEL represent it.
pub(crate) fn field_number(number: u32) -> i32 {
    i32::try_from(number).unwrap_or(i32::MAX)
}

fn path_element_named(field: &FieldDescriptor, name: String) -> FieldPathElement {
    FieldPathElement {
        field_number: Some(field_number(field.number())),
        field_name: Some(name),
        field_type: Some(proto_type(field)),
        ..Default::default()
    }
}

/// The path element naming a oneof, which carries only the name.
pub(crate) fn oneof_path_element(name: &str) -> FieldPathElement {
    FieldPathElement {
        field_name: Some(name.to_owned()),
        ..Default::default()
    }
}

/// The message type a field holds, if it is message-typed.
pub(crate) fn message_type(field: &FieldDescriptor) -> Option<MessageIndex> {
    match field.kind() {
        FieldKind::Singular(SingularKind::Message(idx))
        | FieldKind::List(SingularKind::Message(idx))
        | FieldKind::Map {
            value: SingularKind::Message(idx),
            ..
        } => Some(idx),
        _ => None,
    }
}

/// The `(buf.validate.message)` option on a message, if any.
pub(crate) fn message_rules(message: &MessageDescriptor) -> Option<MessageRules> {
    message
        .options()
        .and_then(|options| options.extension(&crate::validate::MESSAGE))
}

/// The `(buf.validate.field)` option on a field, if any.
pub(crate) fn field_rules(field: &FieldDescriptor) -> Option<FieldRules> {
    field
        .options()
        .and_then(|options| options.extension(&crate::validate::FIELD))
}

/// The `(buf.validate.oneof)` option on a oneof, if any.
pub(crate) fn oneof_rules(oneof: &buffa_descriptor::OneofDescriptor) -> Option<OneofRules> {
    oneof
        .options()
        .and_then(|options| options.extension(&crate::validate::ONEOF))
}

/// The `(buf.validate.predefined)` option on a field of a rules message, if
/// any.
pub(crate) fn predefined_rules(field: &FieldDescriptor) -> Option<PredefinedRules> {
    field
        .options()
        .and_then(|options| options.extension(&crate::validate::PREDEFINED))
}

/// The fields of well-known types the rules read through a runtime during validation.
pub(crate) mod wkt {
    use crate::protobuf::{Field, Kind, Runtime, Scalar, Singular};

    /// `google.protobuf.Any.type_url`.
    pub(crate) const fn any_type_url<R: Runtime>() -> Field<R> {
        Field::well_known(
            1,
            "type_url",
            Kind::Singular(Singular::Scalar(Scalar::String)),
        )
    }

    // Small hack, since the field numbers are the same, and they could never
    // be changed, we use the same Fields for duration and timestamp.

    /// `seconds` of `google.protobuf.Duration` and `Timestamp`.
    pub(crate) const fn seconds<R: Runtime>() -> Field<R> {
        Field::well_known(
            1,
            "seconds",
            Kind::Singular(Singular::Scalar(Scalar::Int64)),
        )
    }

    /// `nanos` of `google.protobuf.Duration` and `Timestamp`.
    pub(crate) const fn nanos<R: Runtime>() -> Field<R> {
        Field::well_known(2, "nanos", Kind::Singular(Singular::Scalar(Scalar::Int32)))
    }

    /// `google.protobuf.FieldMask.paths`.
    pub(crate) const fn field_mask_paths<R: Runtime>() -> Field<R> {
        Field::well_known(1, "paths", Kind::List(Singular::Scalar(Scalar::String)))
    }
}

/// The fields of the `buf.validate` schema that rule paths name, resolved
/// once as path elements for error messages.
pub(crate) struct Schema {
    pub field_rules: MessageIndex,
    pub required: FieldPathElement,
    pub cel: FieldPathElement,
    pub cel_expression: FieldPathElement,
    pub repeated: FieldPathElement,
    pub repeated_items: FieldPathElement,
    pub map: FieldPathElement,
    pub map_keys: FieldPathElement,
    pub map_values: FieldPathElement,
}

impl Schema {
    pub(crate) fn new(pool: &DescriptorPool) -> Self {
        let element = |message: &str, field: &str| {
            let message = pool
                .message_by_name(message)
                .unwrap_or_else(|| panic!("{message} is in the embedded descriptor set"));
            path_element(
                message
                    .field_by_name(field)
                    .unwrap_or_else(|| panic!("{}.{field} exists", message.full_name())),
            )
        };
        Self {
            field_rules: pool
                .message_index(FIELD_RULES)
                .expect("FieldRules is in the embedded descriptor set"),
            required: element(FIELD_RULES, "required"),
            cel: element(FIELD_RULES, "cel"),
            cel_expression: element(FIELD_RULES, "cel_expression"),
            repeated: element(FIELD_RULES, "repeated"),
            repeated_items: element("buf.validate.RepeatedRules", "items"),
            map: element(FIELD_RULES, "map"),
            map_keys: element("buf.validate.MapRules", "keys"),
            map_values: element("buf.validate.MapRules", "values"),
        }
    }

    /// The `FieldRules` field holding the standard rules of one type, such as
    /// `FieldRules.string`.
    pub(crate) fn type_element(&self, pool: &DescriptorPool, name: &str) -> FieldPathElement {
        let field_rules = pool.message(self.field_rules);
        path_element(
            field_rules
                .field_by_name(name)
                .unwrap_or_else(|| panic!("FieldRules.{name} exists")),
        )
    }
}

/// The descriptors a validator reflects over.
pub(crate) struct Descriptors {
    pub pool: Arc<DescriptorPool>,
    pub schema: Schema,
}

impl Descriptors {
    /// Descriptors over a set of files that includes the `buf.validate`
    /// schema, which is expected to link.
    pub(crate) fn from_set(set: FileDescriptorSet) -> Self {
        let pool = DescriptorPool::new(set).expect("the descriptor set links");
        let schema = Schema::new(&pool);
        Self {
            pool: Arc::new(pool),
            schema,
        }
    }

    /// Adds the files of a set, skipping files already known by name.
    pub(crate) fn add_file_set(&mut self, set: FileDescriptorSet) -> Result<(), String> {
        // The pool is shared with nothing that outlives a validation, so
        // this rarely copies. When it does, the copy is the new pool and the
        // old one dies with its last borrower.
        Arc::make_mut(&mut self.pool)
            .add_file_descriptor_set(set)
            .map_err(|error| error.to_string())
    }
}
