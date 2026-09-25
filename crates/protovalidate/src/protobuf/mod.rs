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
//! A runtime plugs in by implementing these and calling
//! [`Validator::validate_message`](crate::Validator::validate_message).
//! Each field the validator asks for is described by a [`Field`], resolved
//! from the descriptors registered with the validator.
//!
//! A read fails with the runtime's own [`Runtime::Error`], which ends the
//! validation and reaches the caller as [`Error::Read`](crate::Error::Read).
//! A native runtime whose reads cannot fail should use [`Infallible`].
//!
//! [`Infallible`]: std::convert::Infallible

use std::borrow::Cow;
use std::ops::ControlFlow;

mod field;

pub use field::{Field, Kind, Scalar, Singular};

/// A Protobuf runtime adapter to read from a message during validation.
pub trait Runtime: Sized {
    /// What a read fails with.
    type Error;
    /// Whatever the runtime needs in order to read a message of one type,
    /// such as which attribute holds each field.
    type MessageType: Clone;
    /// A message.
    type Message<'a>: Message<Self>;
    /// A repeated field's elements.
    type List<'a>: List<Self>;
    /// A map field's entries.
    type Map<'a>: Map<Self>;
}

/// The runtime's side of one validation. It provides the message to
/// validate and looks up message types by name.
pub trait Reader<R: Runtime> {
    /// Looks up the message type named `full_name`, which is fully
    /// qualified and has no leading dot.
    ///
    /// # Errors
    ///
    /// The runtime does not know the type.
    fn resolve(&self, full_name: &str) -> Result<R::MessageType, R::Error>;

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

    /// The field's value, or its type's default when it is not set. `None`
    /// when the message has no such field, as when its runtime knows an
    /// older schema than the validator. The returned value may borrow from
    /// `field`: a nested message keeps a reference to the message type
    /// stored there.
    ///
    /// # Errors
    ///
    /// The runtime could not read the field.
    fn get<'f>(&'f self, field: &'f Field<R>) -> Result<Option<Val<'f, R>>, R::Error>;

    /// The serialized message, which the CEL runtime parses when a rule
    /// binds the message, or one of its repeated or map fields, to `this`.
    ///
    /// # Errors
    ///
    /// The runtime could not serialize the message.
    fn encode(&self) -> Result<Vec<u8>, R::Error>;
}

/// A repeated field.
// A length that may fail to be read has no `is_empty` to pair with.
#[allow(clippy::len_without_is_empty)]
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
#[allow(clippy::len_without_is_empty)]
pub trait Map<R: Runtime> {
    /// The number of entries.
    ///
    /// # Errors
    ///
    /// The runtime could not count them.
    fn len(&self) -> Result<usize, R::Error>;

    /// Visits each entry, as key then value, until `f` breaks. A key has
    /// one of the scalar types Protobuf allows for map keys.
    ///
    /// # Errors
    ///
    /// The runtime could not read an entry, which stops the visit.
    fn for_each<F>(&self, f: F) -> Result<(), R::Error>
    where
        F: FnMut(Val<'_, R>, Val<'_, R>) -> ControlFlow<()>;
}

/// A runtime for tests that never read a message.
#[cfg(test)]
pub(crate) mod testing {
    use std::convert::Infallible;
    use std::ops::ControlFlow;

    use super::{Field, List, Map, Message, Reader, Runtime, Val};

    /// A runtime whose message types carry no information. It serves as
    /// its own reader.
    pub(crate) struct Untyped;

    impl Runtime for Untyped {
        type Error = Infallible;
        type MessageType = ();
        type Message<'a> = Unread;
        type List<'a> = Unread;
        type Map<'a> = Unread;
    }

    impl Reader<Untyped> for Untyped {
        fn resolve(&self, _full_name: &str) -> Result<(), Infallible> {
            Ok(())
        }

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

        fn get<'f>(
            &'f self,
            _field: &'f Field<Untyped>,
        ) -> Result<Option<Val<'f, Untyped>>, Infallible> {
            unreachable!("the test does not read a message")
        }

        fn encode(&self) -> Result<Vec<u8>, Infallible> {
            unreachable!("the test does not read a message")
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

        fn for_each<F>(&self, _f: F) -> Result<(), Infallible>
        where
            F: FnMut(Val<'_, Untyped>, Val<'_, Untyped>) -> ControlFlow<()>,
        {
            unreachable!("the test does not read a map")
        }
    }
}
