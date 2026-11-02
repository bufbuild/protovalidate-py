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

//! How the validator reads messages, independently of the runtime that
//! holds them.
//!
//! A [`Runtime`] names the views a runtime gives out for a message, a
//! repeated field and a map, and those views implement [`Message`],
//! [`List`] and [`Map`]. Validation is generic over the runtime, so a
//! message is read in place without copying, except when it must be
//! encoded for CEL.
//!
//! To support a runtime, implement these traits, along with a [`Resolver`]
//! that looks up the runtime's message types when rules are compiled and a
//! [`Reader`] that provides each message to validate. The validator
//! describes each field it reads with a [`Field`] and each oneof with a
//! [`Oneof`], both resolved from the descriptors registered with it.
//!
//! A read fails with the runtime's own [`Runtime::Error`], which ends the
//! validation and reaches the caller as [`Error::Read`](crate::Error::Read).
//! A native runtime whose reads cannot fail should use [`Infallible`].
//!
//! [`Infallible`]: std::convert::Infallible

use std::borrow::Cow;
use std::ops::ControlFlow;

mod field;
pub mod kind;
mod oneof;

pub use field::{Field, Kind, Scalar, Singular};
pub use kind::{ValueKind, ValueKindId};
pub use oneof::{Oneof, which_oneof_by_has};

/// The resolver for a runtime whose message and field types hold no data.
impl<R: Runtime<MessageType = (), FieldType = ()>> Resolver<R> for () {
    fn resolve(&self, _full_name: &str) -> Result<(), R::Error> {
        Ok(())
    }

    fn resolve_field(&self, _message_type: &(), _number: u32) -> Result<(), R::Error> {
        Ok(())
    }
}

/// A Protobuf runtime adapter to read from a message during validation.
pub trait Runtime: Sized {
    /// What a read fails with.
    type Error;
    /// Whatever the runtime needs in order to read a message of one type,
    /// such as which attribute holds each field.
    type MessageType: Clone;
    /// Whatever the runtime needs in order to read one field of a message.
    /// It is resolved once, when the rules are compiled, and stored in the
    /// [`Field`].
    type FieldType: Clone;
    /// A message.
    type Message<'a>: Message<Self>;
    /// A repeated field's elements.
    type List<'a>: List<Self>;
    /// A map field's entries.
    type Map<'a>: Map<Self>;
}

/// Looks up message types by name when their rules are compiled. Each type
/// is looked up once, and its compiled rules keep the result.
pub trait Resolver<R: Runtime> {
    /// Looks up the message type named `full_name`, which is fully
    /// qualified and has no leading dot.
    ///
    /// # Errors
    ///
    /// The runtime does not know the type.
    fn resolve(&self, full_name: &str) -> Result<R::MessageType, R::Error>;

    /// Looks up the field numbered `number` in a message type.
    ///
    /// # Errors
    ///
    /// The runtime does not know the field.
    fn resolve_field(
        &self,
        message_type: &R::MessageType,
        number: u32,
    ) -> Result<R::FieldType, R::Error>;
}

/// Provides the message for one validation, and resolves message types if
/// rules are compiled during validation.
pub trait Reader<R: Runtime>: Resolver<R> {
    /// Returns the message to validate. `message_type` is the resolved type
    /// of that message.
    ///
    /// # Errors
    ///
    /// The runtime could not read the message as that type.
    fn message<'a>(&'a self, message_type: &'a R::MessageType) -> Result<R::Message<'a>, R::Error>;
}

/// A field's value, borrowed from the message that holds it.
///
/// Numbers are widened to their equivalent CEL types.
#[non_exhaustive]
pub enum Val<'a, R: Runtime> {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Double(f64),
    Enum(i32),
    String(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Message(R::Message<'a>),
    List(R::List<'a>),
    Map(R::Map<'a>),
}

/// A message being validated.
pub trait Message<R: Runtime> {
    /// Whether the field is set. Asked only of a field that
    /// [tracks presence](Field::has_presence).
    ///
    /// # Errors
    ///
    /// The runtime could not tell.
    fn has(&self, field: &Field<R>) -> Result<bool, R::Error>;

    /// Reads a field as the kind `K`.
    ///
    /// The validator only calls this with the field's own kind, as given by
    /// [`Field::kind`]. If the field is not set, the result is the default
    /// value for its type, and an unset message, list or map is empty. If
    /// the message has no such field, as when the runtime has an older
    /// schema than the validator, the result is `None`. The returned value
    /// may borrow from `field`, for example to refer to the message type
    /// stored there.
    ///
    /// An implementation typically matches on [`K::ID`](ValueKind::ID) and
    /// passes the value it reads to the constructor for that kind, such as
    /// [`ValueKind::from_double`]. A runtime that reads every field as a
    /// [`Val`] can call [`ValueKind::from_val`] instead.
    ///
    /// # Errors
    ///
    /// The runtime could not read the field.
    fn get<'f, K: ValueKind<R>>(
        &'f self,
        field: &'f Field<R>,
    ) -> Result<Option<K::Value<'f>>, R::Error>;

    /// The field number of the member of `oneof` that is set, or `None` if
    /// no member is set. A number that is not one of its
    /// [members](Oneof::members), such as a member added in a newer schema
    /// than the validator's, is treated as a member without rules.
    ///
    /// The default implementation calls [`has`](Self::has) on each member
    /// in turn, as [`which_oneof_by_has`] does. A runtime that tracks which
    /// member is set should override it.
    ///
    /// # Errors
    ///
    /// The runtime could not determine which member is set.
    fn which_oneof(&self, oneof: &Oneof<R>) -> Result<Option<u32>, R::Error> {
        which_oneof_by_has(self, oneof)
    }

    /// The serialized message, which the CEL runtime parses when a rule
    /// binds the message, or one of its repeated or map fields, to `this`.
    ///
    /// # Errors
    ///
    /// The runtime could not serialize the message.
    fn encode(&self) -> Result<impl AsRef<[u8]>, R::Error>;
}

/// A repeated field.
// A length that may fail to be read has no `is_empty` to pair with.
#[expect(clippy::len_without_is_empty)]
pub trait List<R: Runtime> {
    /// The number of elements.
    ///
    /// # Errors
    ///
    /// The runtime could not count them.
    fn len(&self) -> Result<usize, R::Error>;

    /// The element at `index`, `None` past the end.
    ///
    /// # Errors
    ///
    /// The runtime could not read the element.
    fn get(&self, index: usize) -> Result<Option<Val<'_, R>>, R::Error>;
}

/// A map field.
// As for `List`.
#[expect(clippy::len_without_is_empty)]
pub trait Map<R: Runtime> {
    /// The number of entries.
    ///
    /// # Errors
    ///
    /// The runtime could not count them.
    fn len(&self) -> Result<usize, R::Error>;

    /// Calls `f` with the key and value of each entry until `f` returns
    /// [`ControlFlow::Break`], and returns the value it broke with. The key
    /// is passed as a function that `f` calls only if it needs the key, so
    /// a runtime should do the work of reading the key in that function.
    /// The key has one of the scalar types that Protobuf allows for map
    /// keys.
    ///
    /// # Errors
    ///
    /// The runtime could not read an entry, which stops the visit.
    fn for_each<B, F>(&self, f: F) -> Result<ControlFlow<B>, R::Error>
    where
        F: for<'k> FnMut(Key<'k, R>, Val<'k, R>) -> ControlFlow<B>;
}

/// A map entry's key, which is only read when the function is called.
pub type Key<'k, R> = &'k dyn Fn() -> Result<Val<'k, R>, <R as Runtime>::Error>;

/// Reads a field of any kind as a [`Val`], by calling [`Message::get`] with
/// the field's kind. The validator uses this on the code paths that handle
/// every kind of field.
pub(crate) fn read<'f, R: Runtime>(
    message: &'f R::Message<'_>,
    field: &'f Field<R>,
) -> Result<Option<Val<'f, R>>, R::Error> {
    Ok(match field.kind() {
        Kind::Singular(Singular::Scalar(scalar)) => match scalar {
            Scalar::Bool => message.get::<kind::Bool>(field)?.map(Val::Bool),
            Scalar::Int32
            | Scalar::Int64
            | Scalar::Sint32
            | Scalar::Sint64
            | Scalar::Sfixed32
            | Scalar::Sfixed64 => message.get::<kind::Int>(field)?.map(Val::Int),
            Scalar::Uint32 | Scalar::Uint64 | Scalar::Fixed32 | Scalar::Fixed64 => {
                message.get::<kind::Uint>(field)?.map(Val::Uint)
            }
            Scalar::Float | Scalar::Double => message.get::<kind::Double>(field)?.map(Val::Double),
            Scalar::String => message.get::<kind::Str>(field)?.map(Val::String),
            Scalar::Bytes => message.get::<kind::Bytes>(field)?.map(Val::Bytes),
        },
        Kind::Singular(Singular::Enum) => message.get::<kind::Enum>(field)?.map(Val::Enum),
        Kind::Singular(Singular::Message) => message.get::<kind::Message>(field)?.map(Val::Message),
        Kind::List(_) => message.get::<kind::List>(field)?.map(Val::List),
        Kind::Map { .. } => message.get::<kind::Map>(field)?.map(Val::Map),
    })
}

/// A runtime for tests that never read a message.
#[cfg(test)]
pub(crate) mod testing {
    use std::convert::Infallible;
    use std::ops::ControlFlow;

    use super::{Field, List, Map, Message, Reader, Resolver, Runtime, Val, ValueKind};

    /// A runtime whose message types hold no data. It is its own resolver
    /// and reader.
    pub(crate) struct Untyped;

    impl Runtime for Untyped {
        type Error = Infallible;
        type MessageType = ();
        type FieldType = ();
        type Message<'a> = Unread;
        type List<'a> = Unread;
        type Map<'a> = Unread;
    }

    impl Resolver<Untyped> for Untyped {
        fn resolve(&self, _full_name: &str) -> Result<(), Infallible> {
            Ok(())
        }

        fn resolve_field(&self, _message_type: &(), _number: u32) -> Result<(), Infallible> {
            Ok(())
        }
    }

    impl Reader<Untyped> for Untyped {
        fn message<'a>(&'a self, _message_type: &'a ()) -> Result<Unread, Infallible> {
            Ok(Unread)
        }
    }

    /// A message, list or map that the test never reads.
    pub(crate) struct Unread;

    impl Message<Untyped> for Unread {
        fn has(&self, _field: &Field<Untyped>) -> Result<bool, Infallible> {
            unreachable!("the test does not read a message")
        }

        fn get<'f, K: ValueKind<Untyped>>(
            &'f self,
            field: &'f Field<Untyped>,
        ) -> Result<Option<K::Value<'f>>, Infallible> {
            let _ = field;
            unreachable!("the test does not read a message")
        }

        #[expect(unreachable_code, reason = "the hidden type must be named")]
        fn encode(&self) -> Result<impl AsRef<[u8]>, Infallible> {
            Ok::<&[u8], _>(unreachable!("the test does not read a message"))
        }
    }

    impl List<Untyped> for Unread {
        fn len(&self) -> Result<usize, Infallible> {
            unreachable!("the test does not read a list")
        }

        fn get(&self, _index: usize) -> Result<Option<Val<'_, Untyped>>, Infallible> {
            unreachable!("the test does not read a list")
        }
    }

    impl Map<Untyped> for Unread {
        fn len(&self) -> Result<usize, Infallible> {
            unreachable!("the test does not read a map")
        }

        fn for_each<B, F>(&self, _f: F) -> Result<ControlFlow<B>, Infallible>
        where
            F: for<'k> FnMut(super::Key<'k, Untyped>, Val<'k, Untyped>) -> ControlFlow<B>,
        {
            unreachable!("the test does not read a map")
        }
    }
}
