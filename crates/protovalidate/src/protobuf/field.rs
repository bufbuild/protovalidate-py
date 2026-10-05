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

//! What the validator tells a runtime about a field it asks for.

use std::borrow::Cow;
use std::fmt;

use buffa::editions::FieldPresence;
use buffa_descriptor::{FieldDescriptor, FieldKind, ScalarType, SingularKind};

use super::Runtime;

/// A field of the message being validated, as the validator describes it
/// when asking a [`Message`](super::Message) for it.
///
/// The description is resolved from the descriptors registered with the
/// validator, and provided to a runtime.
pub struct Field<R: Runtime> {
    number: u32,
    name: Cow<'static, str>,
    kind: Kind,
    message_type: Option<R::MessageType>,
    has_presence: bool,
}

impl<R: Runtime> Clone for Field<R> {
    fn clone(&self) -> Self {
        Self {
            number: self.number,
            name: self.name.clone(),
            kind: self.kind,
            message_type: self.message_type.clone(),
            has_presence: self.has_presence,
        }
    }
}

impl<R: Runtime> fmt::Debug for Field<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("number", &self.number)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("has_presence", &self.has_presence)
            .finish_non_exhaustive()
    }
}

impl<R: Runtime> PartialEq for Field<R>
where
    R::MessageType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
            && self.name == other.name
            && self.kind == other.kind
            && self.message_type == other.message_type
            && self.has_presence == other.has_presence
    }
}

impl<R: Runtime> Eq for Field<R> where R::MessageType: Eq {}

impl<R: Runtime> Field<R> {
    pub(crate) fn from_descriptor(
        field: &FieldDescriptor,
        message_type: Option<R::MessageType>,
    ) -> Self {
        let kind = match field.kind() {
            FieldKind::Singular(value) => Kind::Singular(singular(value)),
            FieldKind::List(element) => Kind::List(singular(element)),
            FieldKind::Map { key, value } => Kind::Map {
                key: scalar(key),
                value: singular(value),
            },
        };
        Self {
            number: field.number(),
            name: Cow::Owned(field.name().to_owned()),
            kind,
            message_type,
            has_presence: matches!(kind, Kind::Singular(_))
                && field.presence() != FieldPresence::Implicit,
        }
    }

    /// A field of a well-known type, which the rules read directly. The
    /// well-known types are proto3, so they do not track presence.
    pub(crate) const fn well_known(number: u32, name: &'static str, kind: Kind) -> Self {
        Self {
            number,
            name: Cow::Borrowed(name),
            kind,
            message_type: None,
            has_presence: false,
        }
    }

    /// The field number.
    #[must_use]
    pub fn number(&self) -> u32 {
        self.number
    }

    /// The field's name in the `.proto` source.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How the field holds its values.
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The resolved type of the messages this field holds, whether as a
    /// single value, as list elements or as map values. `None` if the field
    /// does not hold messages.
    #[must_use]
    pub fn message_type(&self) -> Option<&R::MessageType> {
        self.message_type.as_ref()
    }

    /// Whether the field tracks presence, as a singular field that is
    /// `optional`, a oneof member, or a message does. Only such a field is
    /// asked [`Message::has`](super::Message::has); any other field counts
    /// as set when it is not its type's default.
    #[must_use]
    pub fn has_presence(&self) -> bool {
        self.has_presence
    }
}

/// How a field holds its values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One value.
    Singular(Singular),
    /// A repeated field.
    List(Singular),
    /// A map field.
    Map { key: Scalar, value: Singular },
}

/// The type of one value, whether of a singular field, a repeated element
/// or a map value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Singular {
    Scalar(Scalar),
    /// An enum, read as its number.
    Enum,
    /// A message, of the type [`Field::message_type`] names.
    Message,
}

/// A Protobuf scalar type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scalar {
    Bool,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Float,
    Double,
    String,
    Bytes,
}

fn singular(kind: SingularKind) -> Singular {
    match kind {
        SingularKind::Scalar(scalar_type) => Singular::Scalar(scalar(scalar_type)),
        SingularKind::Enum(_) => Singular::Enum,
        SingularKind::Message(_) => Singular::Message,
    }
}

fn scalar(scalar: ScalarType) -> Scalar {
    match scalar {
        ScalarType::Bool => Scalar::Bool,
        ScalarType::Int32 => Scalar::Int32,
        ScalarType::Int64 => Scalar::Int64,
        ScalarType::Uint32 => Scalar::Uint32,
        ScalarType::Uint64 => Scalar::Uint64,
        ScalarType::Sint32 => Scalar::Sint32,
        ScalarType::Sint64 => Scalar::Sint64,
        ScalarType::Fixed32 => Scalar::Fixed32,
        ScalarType::Fixed64 => Scalar::Fixed64,
        ScalarType::Sfixed32 => Scalar::Sfixed32,
        ScalarType::Sfixed64 => Scalar::Sfixed64,
        ScalarType::Float => Scalar::Float,
        ScalarType::Double => Scalar::Double,
        ScalarType::String => Scalar::String,
        ScalarType::Bytes => Scalar::Bytes,
    }
}
