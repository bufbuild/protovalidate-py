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

use std::cell::{OnceCell, RefCell};
use std::ops::ControlFlow;
use std::sync::Arc;

use protovalidate::protobuf::{
    Field, Kind, List, Map, Message, Reader, Runtime, Scalar, Singular, Val,
};
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

    fn find(&self, number: u32) -> Option<(usize, &FieldInfo)> {
        let slot = self
            .fields
            .binary_search_by_key(&number, |field| field.number)
            .ok()?;
        Some((slot, &self.fields[slot]))
    }
}

/// Where message types are looked up by name.
pub(crate) enum TypeSource<'py> {
    /// For protobuf-py, a `Registry` containing every registered file.
    Registry(&'py Bound<'py, PyAny>),
    /// For google.protobuf, the descriptor of the message being validated.
    /// Its pool contains every type reachable from that message.
    Descriptor(&'py Bound<'py, PyAny>),
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

impl Reader<PyRuntime> for Ctx<'_> {
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

    fn message<'a>(
        &'a self,
        message_type: &'a Arc<TypeInfo>,
    ) -> Result<MessageView<'a>, ReadError> {
        Ok(MessageView::new(
            self,
            self.message.clone(),
            Some(message_type),
        ))
    }
}

/// A message, read through its Python object.
pub(crate) struct MessageView<'a> {
    ctx: &'a Ctx<'a>,
    /// `None` for a protobuf-py message field that is not set, which reads
    /// as a message with nothing set.
    info: Option<&'a TypeInfo>,
    object: Bound<'a, PyAny>,
    /// Field values already fetched, by position in `info.fields`, so a
    /// value is read once and borrowed from for as long as the view lives.
    /// When the view is dropped, the buffer is emptied and returned to the
    /// context.
    values: Values,
}

impl<'a> MessageView<'a> {
    /// Creates a view of `object`. `info` describes its type.
    fn new(ctx: &'a Ctx<'a>, object: Bound<'a, PyAny>, info: Option<&'a TypeInfo>) -> Self {
        let info = if object.is_none() { None } else { info };
        let values = info.map_or(0, |info| info.fields.len());
        Self {
            ctx,
            info,
            object,
            values: ctx.take_values(values),
        }
    }

    /// The field's value, `None` when it is not set and the runtime has no
    /// default object for it.
    fn value(&self, slot: usize, field: &FieldInfo) -> PyResult<Option<&Bound<'a, PyAny>>> {
        let cell = &self.values[slot];
        if cell.get().is_none() {
            let fetched = field.fetch(&self.object, self.ctx.constants)?;
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

impl Message<PyRuntime> for MessageView<'_> {
    #[inline]
    fn encode(&self) -> Result<Vec<u8>, ReadError> {
        // An unset protobuf-py message field is `None`, and reads as a
        // message with nothing set, which encodes to nothing.
        if self.object.is_none() {
            return Ok(Vec::new());
        }
        let bytes = self
            .ctx
            .runtime
            .serialize(&self.object, self.ctx.constants)?;
        Ok(bytes.as_bytes().to_vec())
    }

    #[inline]
    fn has(&self, field: &Field<PyRuntime>) -> Result<bool, ReadError> {
        let Some(info) = self.info else {
            return Ok(false);
        };
        let Some((slot, stored)) = info.find(field.number()) else {
            return Ok(false);
        };
        Ok(self.ctx.runtime.is_set(
            &self.object,
            stored,
            field.kind(),
            || Ok(self.value(slot, stored)?.is_some()),
            self.ctx.constants,
        )?)
    }

    #[inline]
    fn get<'f>(
        &'f self,
        field: &'f Field<PyRuntime>,
    ) -> Result<Option<Val<'f, PyRuntime>>, ReadError> {
        let message_type = field.message_type().map(Arc::as_ref);
        let Some(info) = self.info else {
            // Nothing is set: every field reads as its default.
            return Ok(Some(convert(self.ctx, field.kind(), None, message_type)?));
        };
        let Some((slot, stored)) = info.find(field.number()) else {
            return Ok(None);
        };
        Ok(Some(convert(
            self.ctx,
            field.kind(),
            self.value(slot, stored)?,
            message_type,
        )?))
    }
}

/// A field's value as the validator sees it; the default when `value` is
/// `None`.
fn convert<'a>(
    ctx: &'a Ctx<'a>,
    kind: Kind,
    value: Option<&'a Bound<'a, PyAny>>,
    message_type: Option<&'a TypeInfo>,
) -> PyResult<Val<'a, PyRuntime>> {
    Ok(match kind {
        Kind::Singular(kind) => singular(ctx, kind, value, message_type)?,
        Kind::List(element) => Val::List(ListView {
            ctx,
            element,
            object: value,
            message_type,
            items: OnceCell::new(),
        }),
        Kind::Map {
            key,
            value: element,
        } => Val::Map(MapView {
            ctx,
            key,
            element,
            object: value,
            message_type,
        }),
    })
}

fn singular<'a>(
    ctx: &'a Ctx<'a>,
    kind: Singular,
    value: Option<&'a Bound<'a, PyAny>>,
    message_type: Option<&'a TypeInfo>,
) -> PyResult<Val<'a, PyRuntime>> {
    Ok(match kind {
        Singular::Message => {
            if value.is_some() && message_type.is_none() {
                return Err(PyTypeError::new_err(
                    "a message field was read without its message type",
                ));
            }
            let object = value
                .cloned()
                .unwrap_or_else(|| ctx.py.None().into_bound(ctx.py));
            Val::Message(MessageView::new(ctx, object, message_type))
        }
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
    #[inline]
    fn len(&self) -> Result<usize, ReadError> {
        Ok(match (self.items.get(), self.object) {
            (Some(items), _) => items.len(),
            (None, Some(object)) => object.len()?,
            (None, None) => 0,
        })
    }

    #[inline]
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
    #[inline]
    fn len(&self) -> Result<usize, ReadError> {
        Ok(match self.object {
            Some(object) => object.len()?,
            None => 0,
        })
    }

    #[inline]
    fn for_each<F>(&self, mut f: F) -> Result<(), ReadError>
    where
        F: FnMut(Val<'_, PyRuntime>, Val<'_, PyRuntime>) -> ControlFlow<()>,
    {
        let Some(object) = self.object else {
            return Ok(());
        };
        for entry in self.ctx.runtime.map_entries(object, self.ctx.constants)? {
            let (k, v) = entry?;
            let flow = f(
                scalar_value(self.key, &k)?,
                singular(self.ctx, self.element, Some(&v), self.message_type)?,
            );
            if flow.is_break() {
                return Ok(());
            }
        }
        Ok(())
    }
}
