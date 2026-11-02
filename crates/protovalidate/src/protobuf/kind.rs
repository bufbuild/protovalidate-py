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

//! The kinds of value that [`Message::get`](super::Message::get) reads,
//! such as [`Int`] or [`Message`].

use std::borrow::Cow;

use super::{Runtime, Val};

/// Identifies a [`ValueKind`]. A runtime's `get` matches on
/// [`ValueKind::ID`] to decide how to read the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKindId {
    Bool,
    Int,
    Uint,
    Double,
    Enum,
    Str,
    Bytes,
    Message,
    List,
    Map,
}

/// A kind of field value that [`Message::get`](super::Message::get) reads.
///
/// A runtime's `get` passes the value it reads to the constructor for the
/// value's kind, such as [`from_double`](Self::from_double), and returns
/// the result. Pass the value to its constructor as soon as it is read,
/// rather than wrapping it in a `Result`, `Option` or [`Val`] first.
pub trait ValueKind<R: Runtime> {
    /// The type of the value read for this kind.
    type Value<'a>;

    /// The identifier of this kind.
    const ID: ValueKindId;

    /// Returns `Some(value)` if this is the `bool` kind, and `None` otherwise.
    #[must_use]
    fn from_bool<'a>(value: bool) -> Option<Self::Value<'a>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the signed integer kind, and `None` otherwise.
    #[must_use]
    fn from_int<'a>(value: i64) -> Option<Self::Value<'a>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the unsigned integer kind, and `None` otherwise.
    #[must_use]
    fn from_uint<'a>(value: u64) -> Option<Self::Value<'a>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the floating-point kind, and `None` otherwise.
    #[must_use]
    fn from_double<'a>(value: f64) -> Option<Self::Value<'a>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the enum kind, and `None` otherwise.
    #[must_use]
    fn from_enum<'a>(value: i32) -> Option<Self::Value<'a>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the `string` kind, and `None` otherwise.
    #[must_use]
    fn from_str(value: Cow<'_, str>) -> Option<Self::Value<'_>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the `bytes` kind, and `None` otherwise.
    #[must_use]
    fn from_bytes(value: Cow<'_, [u8]>) -> Option<Self::Value<'_>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the message kind, and `None` otherwise.
    #[must_use]
    fn from_message(value: R::Message<'_>) -> Option<Self::Value<'_>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the repeated field kind, and `None` otherwise.
    #[must_use]
    fn from_list(value: R::List<'_>) -> Option<Self::Value<'_>> {
        let _ = value;
        None
    }

    /// Returns `Some(value)` if this is the map kind, and `None` otherwise.
    #[must_use]
    fn from_map(value: R::Map<'_>) -> Option<Self::Value<'_>> {
        let _ = value;
        None
    }

    /// Returns the value inside `val` if `val` is of this kind, and `None`
    /// otherwise. This is for a runtime that reads every field as a
    /// [`Val`].
    #[must_use]
    fn from_val(val: Val<'_, R>) -> Option<Self::Value<'_>> {
        match val {
            Val::Bool(value) => Self::from_bool(value),
            Val::Int(value) => Self::from_int(value),
            Val::Uint(value) => Self::from_uint(value),
            Val::Double(value) => Self::from_double(value),
            Val::Enum(value) => Self::from_enum(value),
            Val::String(value) => Self::from_str(value),
            Val::Bytes(value) => Self::from_bytes(value),
            Val::Message(value) => Self::from_message(value),
            Val::List(value) => Self::from_list(value),
            Val::Map(value) => Self::from_map(value),
        }
    }
}

macro_rules! kinds {
    ($($(#[$doc:meta])* $name:ident: $from:ident($value:ty);)*) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Copy, Debug)]
            pub struct $name;

            impl<R: Runtime> ValueKind<R> for $name {
                type Value<'a> = $value;

                const ID: ValueKindId = ValueKindId::$name;

                #[inline]
                fn $from<'a>(value: $value) -> Option<Self::Value<'a>> {
                    Some(value)
                }
            }
        )*
    };
}

kinds! {
    /// A `bool` field.
    Bool: from_bool(bool);
    /// A signed integer field of any width or encoding, widened to 64 bits.
    Int: from_int(i64);
    /// An unsigned integer field of any width or encoding, widened to 64 bits.
    Uint: from_uint(u64);
    /// A `float` or `double` field, widened to `f64`.
    Double: from_double(f64);
    /// The numeric value of an enum field.
    Enum: from_enum(i32);
    /// A `string` field.
    Str: from_str(Cow<'a, str>);
    /// A `bytes` field.
    Bytes: from_bytes(Cow<'a, [u8]>);
    /// A singular message field, which is an empty message when it is not
    /// set.
    Message: from_message(R::Message<'a>);
    /// A repeated field, which is empty when it is not set.
    List: from_list(R::List<'a>);
    /// A map field, which is empty when it is not set.
    Map: from_map(R::Map<'a>);
}
