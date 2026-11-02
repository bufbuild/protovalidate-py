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

//! The validator's view of a live Python message.
//!
//! [`PyRuntime`] implements the `protovalidate` reflection traits over
//! protobuf-py and google.protobuf message objects, so a message is
//! validated in place.

use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::ops::ControlFlow;
use std::sync::Arc;

use protovalidate::protobuf::{
    Field, Key, Kind, List, Map, Message, Oneof, Reader, Resolver, Runtime, Scalar, Singular, Val,
    ValueKind, ValueKindId,
};
use pyo3::Borrowed;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

use crate::constants::Constants;
use crate::runtime::{FieldInfo, ProtoRuntime};

/// The Python error a read raised.
type ReadError = Box<PyErr>;

/// The Python runtimes, as the validator reads them.
pub(crate) struct PyRuntime;

impl Runtime for PyRuntime {
    type Error = ReadError;
    type MessageType = Arc<TypeInfo>;
    /// The field's position in its message type's fields, or `None` if the
    /// Python type has no such field.
    type FieldType = Option<usize>;
    type Message<'a> = MessageView<'a>;
    type List<'a> = ListView<'a>;
    type Map<'a> = MapView<'a>;
}

/// Describes how to read each field of one message type from its Python
/// objects.
pub(crate) struct TypeInfo {
    /// Sorted by field number.
    fields: Vec<FieldInfo>,
}

impl TypeInfo {
    fn build(ctx: &Ctx<'_>, descriptor: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut fields = ctx.runtime.fields(ctx.py, descriptor, ctx.constants)?;
        fields.sort_by_key(|field| field.number);
        Ok(Self { fields })
    }

    fn find(&self, number: u32) -> Option<usize> {
        self.fields
            .binary_search_by_key(&number, |field| field.number)
            .ok()
    }
}

/// Where message types are looked up by name.
pub(crate) enum TypeSource<'py> {
    /// For protobuf-py, a `Registry` containing every registered file.
    Registry(&'py Bound<'py, PyAny>),
    /// For google.protobuf, the descriptor of the message being validated.
    /// Its pool contains every type reachable from that message.
    Descriptor(Borrowed<'py, 'py, PyAny>),
}

/// The field values a view has fetched, indexed like its type's field list.
type Values = Box<[OnceCell<Option<Py<PyAny>>>]>;

/// The state of one validation: the message being validated, and what
/// every view needs besides its own object.
pub(crate) struct Ctx<'py> {
    py: Python<'py>,
    runtime: ProtoRuntime,
    constants: &'py Constants,
    message: &'py Bound<'py, PyAny>,
    source: TypeSource<'py>,
    /// Value buffers returned by dropped views, for later views to reuse.
    free_values: RefCell<Vec<Values>>,
    /// Element buffers returned by dropped list views, likewise.
    free_items: RefCell<Vec<Vec<Py<PyAny>>>>,
}

impl<'py> Ctx<'py> {
    pub(crate) fn new(
        py: Python<'py>,
        runtime: ProtoRuntime,
        constants: &'py Constants,
        message: &'py Bound<'py, PyAny>,
        source: TypeSource<'py>,
    ) -> Self {
        Self {
            py,
            runtime,
            constants,
            message,
            source,
            free_values: RefCell::new(Vec::new()),
            free_items: RefCell::new(Vec::new()),
        }
    }

    /// Returns an empty value buffer with at least `len` slots, reusing a
    /// returned one when possible.
    fn take_values(&self, len: usize) -> Values {
        if len == 0 {
            return Values::default();
        }
        let mut free = self.free_values.borrow_mut();
        match free.iter().rposition(|values| values.len() >= len) {
            Some(found) => free.swap_remove(found),
            None => std::iter::repeat_with(OnceCell::new).take(len).collect(),
        }
    }
}

impl Resolver<PyRuntime> for Ctx<'_> {
    fn resolve(&self, full_name: &str) -> Result<Arc<TypeInfo>, ReadError> {
        let descriptor = match self.source {
            TypeSource::Registry(registry) => {
                let descriptor = registry.call_method1(&self.constants.message, (full_name,))?;
                if descriptor.is_none() {
                    return Err(Box::new(PyValueError::new_err(format!(
                        "message type {full_name} is not in the registry"
                    ))));
                }
                descriptor
            }
            TypeSource::Descriptor(descriptor) => descriptor
                .getattr(&self.constants.file)?
                .getattr(&self.constants.pool)?
                .call_method1(&self.constants.find_message_type_by_name, (full_name,))?,
        };
        Ok(Arc::new(TypeInfo::build(self, &descriptor)?))
    }

    fn resolve_field(
        &self,
        message_type: &Arc<TypeInfo>,
        number: u32,
    ) -> Result<Option<usize>, ReadError> {
        Ok(message_type.find(number))
    }
}

impl Reader<PyRuntime> for Ctx<'_> {
    fn message<'a>(
        &'a self,
        message_type: &'a Arc<TypeInfo>,
    ) -> Result<MessageView<'a>, ReadError> {
        Ok(MessageView::new(self, Some((self.message, message_type))))
    }
}

/// A message, read through its Python object.
pub(crate) struct MessageView<'a> {
    ctx: &'a Ctx<'a>,
    /// The object, and how to read its fields. `None` for a protobuf-py
    /// message field that is not set, which reads as a message with
    /// nothing set.
    object: Option<(&'a Bound<'a, PyAny>, &'a TypeInfo)>,
    /// Field values already fetched, by position in the type's fields, so a
    /// value is read once and borrowed from for as long as the view lives.
    /// When the view is dropped, the buffer is emptied and returned to the
    /// context.
    values: Values,
}

impl<'a> MessageView<'a> {
    fn new(ctx: &'a Ctx<'a>, object: Option<(&'a Bound<'a, PyAny>, &'a TypeInfo)>) -> Self {
        let values = object.map_or(0, |(_, info)| info.fields.len());
        Self {
            ctx,
            object,
            values: ctx.take_values(values),
        }
    }

    /// The field's value, `None` when it is not set and the runtime has no
    /// default object for it.
    fn value(
        &self,
        object: &Bound<'a, PyAny>,
        slot: usize,
        field: &FieldInfo,
    ) -> PyResult<Option<&Bound<'a, PyAny>>> {
        let cell = &self.values[slot];
        if cell.get().is_none() {
            let fetched = field.fetch(object, self.ctx.constants)?;
            let _ = cell.set(fetched.map(Bound::unbind));
        }
        Ok(cell
            .get()
            .and_then(Option::as_ref)
            .map(|value| value.bind(self.ctx.py)))
    }
}

impl Drop for MessageView<'_> {
    fn drop(&mut self) {
        for cell in &mut self.values {
            cell.take();
        }
        let values = std::mem::take(&mut self.values);
        if !values.is_empty() {
            self.ctx.free_values.borrow_mut().push(values);
        }
    }
}

impl<'a> MessageView<'a> {
    /// The message object, the field's position, and how to read the field,
    /// or `None` if the message is unset or its type has no such field.
    fn stored(
        &self,
        field: &Field<PyRuntime>,
    ) -> Option<(&'a Bound<'a, PyAny>, usize, &'a FieldInfo)> {
        let (object, info) = self.object?;
        let slot = (*field.field_type())?;
        Some((object, slot, info.fields.get(slot)?))
    }

    /// The field's Python object. Every field of an unset message is
    /// treated as not set.
    fn fetch(&self, field: &Field<PyRuntime>) -> PyResult<Fetched<'_, 'a>> {
        if self.object.is_none() {
            return Ok(Fetched::Unset);
        }
        let Some((object, slot, stored)) = self.stored(field) else {
            return Ok(Fetched::Missing);
        };
        Ok(match self.value(object, slot, stored)? {
            Some(value) => Fetched::Set(value),
            None => Fetched::Unset,
        })
    }

    fn read_bool<T>(
        &self,
        field: &Field<PyRuntime>,
        into: impl FnOnce(bool) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        self.fetch(field)?
            .scalar(false, PyAnyMethods::extract, into)
    }

    fn read_int<T>(
        &self,
        field: &Field<PyRuntime>,
        into: impl FnOnce(i64) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        self.fetch(field)?.scalar(0, PyAnyMethods::extract, into)
    }

    fn read_uint<T>(
        &self,
        field: &Field<PyRuntime>,
        into: impl FnOnce(u64) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        self.fetch(field)?.scalar(0, PyAnyMethods::extract, into)
    }

    fn read_double<T>(
        &self,
        field: &Field<PyRuntime>,
        into: impl FnOnce(f64) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        self.fetch(field)?.scalar(0.0, PyAnyMethods::extract, into)
    }

    fn read_enum<T>(
        &self,
        field: &Field<PyRuntime>,
        into: impl FnOnce(i32) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        self.fetch(field)?.scalar(0, PyAnyMethods::extract, into)
    }

    fn read_str<'f, T>(
        &'f self,
        field: &'f Field<PyRuntime>,
        into: impl FnOnce(Cow<'f, str>) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        self.fetch(field)?.scalar(
            Cow::Borrowed(""),
            |value| Ok(Cow::Borrowed(value.cast::<PyString>()?.to_str()?)),
            into,
        )
    }

    fn read_bytes<'f, T>(
        &'f self,
        field: &'f Field<PyRuntime>,
        into: impl FnOnce(Cow<'f, [u8]>) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        self.fetch(field)?.scalar(
            Cow::Borrowed(&[][..]),
            |value| Ok(Cow::Borrowed(value.cast::<PyBytes>()?.as_bytes())),
            into,
        )
    }

    fn read_message<'f, T>(
        &'f self,
        field: &'f Field<PyRuntime>,
        into: impl FnOnce(MessageView<'f>) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        let message_type = field.message_type().map(Arc::as_ref);
        let value = match self.fetch(field)? {
            Fetched::Set(value) => Some(value),
            Fetched::Unset => None,
            Fetched::Missing => return Ok(None),
        };
        Ok(into(message(self.ctx, value, message_type)?))
    }

    fn read_list<'f, T>(
        &'f self,
        field: &'f Field<PyRuntime>,
        into: impl FnOnce(ListView<'f>) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        let Kind::List(element) = field.kind() else {
            return Ok(None);
        };
        let object = match self.fetch(field)? {
            Fetched::Set(value) => Some(value),
            Fetched::Unset => None,
            Fetched::Missing => return Ok(None),
        };
        Ok(into(ListView {
            ctx: self.ctx,
            element,
            object,
            message_type: field.message_type().map(Arc::as_ref),
            items: OnceCell::new(),
        }))
    }

    fn read_map<'f, T>(
        &'f self,
        field: &'f Field<PyRuntime>,
        into: impl FnOnce(MapView<'f>) -> Option<T>,
    ) -> Result<Option<T>, ReadError> {
        let Kind::Map { key, value } = field.kind() else {
            return Ok(None);
        };
        let object = match self.fetch(field)? {
            Fetched::Set(value) => Some(value),
            Fetched::Unset => None,
            Fetched::Missing => return Ok(None),
        };
        Ok(into(MapView {
            ctx: self.ctx,
            key,
            element: value,
            object,
            message_type: field.message_type().map(Arc::as_ref),
        }))
    }
}

/// Whether a field is set, as found when reading it.
enum Fetched<'v, 'a> {
    /// The field is set to this object.
    Set(&'v Bound<'a, PyAny>),
    /// The field is not set and reads as its type's default value.
    Unset,
    /// The message's type has no such field.
    Missing,
}

impl<'v, 'a> Fetched<'v, 'a> {
    /// Converts the field's object with `extract`, or uses `default` if the
    /// field is not set, and passes the result to `into`.
    fn scalar<T, U>(
        self,
        default: T,
        extract: impl FnOnce(&'v Bound<'a, PyAny>) -> PyResult<T>,
        into: impl FnOnce(T) -> Option<U>,
    ) -> Result<Option<U>, ReadError> {
        Ok(match self {
            Self::Set(value) => into(extract(value)?),
            Self::Unset => into(default),
            Self::Missing => None,
        })
    }
}

impl Message<PyRuntime> for MessageView<'_> {
    fn encode(&self) -> Result<impl AsRef<[u8]>, ReadError> {
        // An unset message is empty, so it encodes to no bytes.
        Ok(match self.object {
            Some((object, _)) => self.ctx.runtime.serialize(object, self.ctx.constants)?,
            None => PyBytes::new(self.ctx.py, &[]),
        })
    }

    fn has(&self, field: &Field<PyRuntime>) -> Result<bool, ReadError> {
        let Some((object, slot, stored)) = self.stored(field) else {
            return Ok(false);
        };
        Ok(self.ctx.runtime.is_set(
            object,
            stored,
            field.kind(),
            || Ok(self.value(object, slot, stored)?.is_some()),
            self.ctx.constants,
        )?)
    }

    fn which_oneof(&self, oneof: &Oneof<PyRuntime>) -> Result<Option<u32>, ReadError> {
        // If the Python type has none of the members, none of them is set.
        let Some((object, _, member)) = oneof.members().iter().find_map(|m| self.stored(m)) else {
            return Ok(None);
        };
        let runtime = self.ctx.runtime;
        let Some(set) = runtime.which_oneof(object, oneof.name(), member, self.ctx.constants)?
        else {
            return Ok(None);
        };
        let set = set.cast_into::<PyString>().map_err(PyErr::from)?;
        let set = set.to_str()?;
        if let Some(member) = oneof.members().iter().find(|m| m.name() == set) {
            return Ok(Some(member.number()));
        }
        // The set member is one that the validator does not know of, so its
        // number comes from the Python type.
        let info = self.object.map(|(_, info)| info);
        let number = info.and_then(|info| {
            info.fields
                .iter()
                .find(|field| field.member_name().bind(self.ctx.py) == set)
                .map(|field| field.number)
        });
        number.map(Some).ok_or_else(|| {
            Box::new(PyValueError::new_err(format!(
                "oneof {} has no member {set}",
                oneof.name()
            )))
        })
    }

    fn get<'f, K: ValueKind<PyRuntime>>(
        &'f self,
        field: &'f Field<PyRuntime>,
    ) -> Result<Option<K::Value<'f>>, ReadError> {
        // `K::ID` is a constant here, so only its arm is compiled. Each read
        // passes the value it builds straight to the constructor, because
        // moving a view through an intermediate `Result` or `Option` would
        // copy it through the stack.
        match K::ID {
            ValueKindId::Bool => self.read_bool(field, K::from_bool),
            ValueKindId::Int => self.read_int(field, K::from_int),
            ValueKindId::Uint => self.read_uint(field, K::from_uint),
            ValueKindId::Double => self.read_double(field, K::from_double),
            ValueKindId::Enum => self.read_enum(field, K::from_enum),
            ValueKindId::Str => self.read_str(field, K::from_str),
            ValueKindId::Bytes => self.read_bytes(field, K::from_bytes),
            ValueKindId::Message => self.read_message(field, K::from_message),
            ValueKindId::List => self.read_list(field, K::from_list),
            ValueKindId::Map => self.read_map(field, K::from_map),
        }
    }
}

/// A message value, which is either the object and its type, or an unset
/// message that reads as an empty message.
fn message<'a>(
    ctx: &'a Ctx<'a>,
    value: Option<&'a Bound<'a, PyAny>>,
    message_type: Option<&'a TypeInfo>,
) -> PyResult<MessageView<'a>> {
    let object = match (value, message_type) {
        (Some(value), Some(info)) => Some((value, info)),
        (Some(_), None) => {
            return Err(PyTypeError::new_err(
                "a message field was read without its message type",
            ));
        }
        (None, _) => None,
    };
    Ok(MessageView::new(ctx, object))
}

fn singular<'a>(
    ctx: &'a Ctx<'a>,
    kind: Singular,
    value: Option<&'a Bound<'a, PyAny>>,
    message_type: Option<&'a TypeInfo>,
) -> PyResult<Val<'a, PyRuntime>> {
    Ok(match kind {
        Singular::Message => Val::Message(message(ctx, value, message_type)?),
        Singular::Enum => Val::Enum(match value {
            Some(value) => value.extract::<i32>()?,
            None => 0,
        }),
        Singular::Scalar(scalar) => match value {
            Some(value) => scalar_value(scalar, value)?,
            None => default(scalar),
        },
    })
}

fn scalar_value<'a>(scalar: Scalar, value: &'a Bound<'a, PyAny>) -> PyResult<Val<'a, PyRuntime>> {
    Ok(match scalar {
        Scalar::Bool => Val::Bool(value.extract()?),
        Scalar::Int32
        | Scalar::Int64
        | Scalar::Sint32
        | Scalar::Sint64
        | Scalar::Sfixed32
        | Scalar::Sfixed64 => Val::Int(value.extract()?),
        Scalar::Uint32 | Scalar::Uint64 | Scalar::Fixed32 | Scalar::Fixed64 => {
            Val::Uint(value.extract()?)
        }
        Scalar::Float | Scalar::Double => Val::Double(value.extract()?),
        Scalar::String => Val::String(value.cast::<PyString>()?.to_str()?.into()),
        Scalar::Bytes => Val::Bytes(value.cast::<PyBytes>()?.as_bytes().into()),
    })
}

fn default<'a>(scalar: Scalar) -> Val<'a, PyRuntime> {
    match scalar {
        Scalar::Bool => Val::Bool(false),
        Scalar::Int32
        | Scalar::Int64
        | Scalar::Sint32
        | Scalar::Sint64
        | Scalar::Sfixed32
        | Scalar::Sfixed64 => Val::Int(0),
        Scalar::Uint32 | Scalar::Uint64 | Scalar::Fixed32 | Scalar::Fixed64 => Val::Uint(0),
        Scalar::Float | Scalar::Double => Val::Double(0.0),
        Scalar::String => Val::String("".into()),
        Scalar::Bytes => Val::Bytes((&[][..]).into()),
    }
}

/// A repeated field.
pub(crate) struct ListView<'a> {
    ctx: &'a Ctx<'a>,
    element: Singular,
    /// `None` for an unset field, which is empty.
    object: Option<&'a Bound<'a, PyAny>>,
    /// The elements' type, if they are messages.
    message_type: Option<&'a TypeInfo>,
    /// The elements, fetched all at once on first access and held here for
    /// values to borrow from.
    items: OnceCell<Vec<Py<PyAny>>>,
}

impl ListView<'_> {
    fn items(&self) -> PyResult<&[Py<PyAny>]> {
        if let Some(items) = self.items.get() {
            return Ok(items);
        }
        let mut items = Vec::new();
        if let Some(object) = self.object {
            items = self.ctx.free_items.borrow_mut().pop().unwrap_or_default();
            self.ctx.runtime.list_items(object, &mut items)?;
        }
        Ok(self.items.get_or_init(|| items))
    }
}

impl Drop for ListView<'_> {
    fn drop(&mut self) {
        let Some(mut items) = self.items.take() else {
            return;
        };
        items.clear();
        if items.capacity() > 0 {
            self.ctx.free_items.borrow_mut().push(items);
        }
    }
}

impl List<PyRuntime> for ListView<'_> {
    fn len(&self) -> Result<usize, ReadError> {
        Ok(match (self.items.get(), self.object) {
            (Some(items), _) => items.len(),
            (None, Some(object)) => object.len()?,
            (None, None) => 0,
        })
    }

    fn get(&self, index: usize) -> Result<Option<Val<'_, PyRuntime>>, ReadError> {
        let Some(item) = self.items()?.get(index) else {
            return Ok(None);
        };
        let item = item.bind(self.ctx.py);
        Ok(Some(singular(
            self.ctx,
            self.element,
            Some(item),
            self.message_type,
        )?))
    }
}

/// A map field.
pub(crate) struct MapView<'a> {
    ctx: &'a Ctx<'a>,
    key: Scalar,
    element: Singular,
    /// `None` for an unset field, which is empty.
    object: Option<&'a Bound<'a, PyAny>>,
    /// The values' type, if they are messages.
    message_type: Option<&'a TypeInfo>,
}

impl Map<PyRuntime> for MapView<'_> {
    fn len(&self) -> Result<usize, ReadError> {
        Ok(match self.object {
            Some(object) => object.len()?,
            None => 0,
        })
    }

    fn for_each<B, F>(&self, mut f: F) -> Result<ControlFlow<B>, ReadError>
    where
        F: for<'k> FnMut(Key<'k, PyRuntime>, Val<'k, PyRuntime>) -> ControlFlow<B>,
    {
        let Some(object) = self.object else {
            return Ok(ControlFlow::Continue(()));
        };
        for entry in self.ctx.runtime.map_entries(object, self.ctx.constants)? {
            let (k, v) = entry?;
            // The key is only converted if the validator reads it.
            let key = || Ok(scalar_value(self.key, &k)?);
            let flow = f(
                &key,
                singular(self.ctx, self.element, Some(&v), self.message_type)?,
            );
            if flow.is_break() {
                return Ok(flow);
            }
        }
        Ok(ControlFlow::Continue(()))
    }
}
