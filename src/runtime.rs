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

//! Adapters on the two Python Protobuf runtimes.

use std::collections::HashSet;

use protovalidate::protobuf::{Kind, Singular};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::iter::BoundDictIterator;
use pyo3::types::{PyBytes, PyDict, PyIterator, PyList, PyString, PyType};

use crate::constants::Constants;

/// Which Python protobuf runtime a message comes from.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtoRuntime {
    /// protobuf-py: message classes carry a `desc` classmethod.
    ProtobufPy,
    /// google.protobuf: messages carry a `DESCRIPTOR` attribute.
    Google,
}

/// A message's runtime together with its resolved descriptor.
pub struct ProtoAdapter {
    pub(crate) runtime: ProtoRuntime,
    descriptor: Py<PyAny>,
}

impl ProtoAdapter {
    /// Resolves a message's runtime, capturing its descriptor.
    pub(crate) fn resolve(message: &Bound<'_, PyAny>, constants: &Constants) -> PyResult<Self> {
        // We skip isinstance in favor of duck typing since it is robust. It is important to call desc
        // on the type rather than the message though.
        if let Ok(descriptor) = message.get_type().call_method0(&constants.desc) {
            return Ok(Self {
                runtime: ProtoRuntime::ProtobufPy,
                descriptor: descriptor.unbind(),
            });
        }
        if let Ok(descriptor) = message.getattr(&constants.descriptor_upper) {
            return Ok(Self {
                runtime: ProtoRuntime::Google,
                descriptor: descriptor.unbind(),
            });
        }
        Err(PyTypeError::new_err("expected a protobuf message"))
    }

    /// The descriptor this adapter was resolved from.
    pub(crate) fn descriptor<'py>(&'py self, py: Python<'py>) -> &'py Bound<'py, PyAny> {
        self.descriptor.bind(py)
    }

    pub(crate) fn clone_ref(&self, py: Python<'_>) -> Self {
        Self {
            runtime: self.runtime,
            descriptor: self.descriptor.clone_ref(py),
        }
    }

    /// The message's fully qualified type name.
    pub(crate) fn type_name<'py>(
        &self,
        py: Python<'py>,
        constants: &Constants,
    ) -> PyResult<Bound<'py, PyString>> {
        let attribute = match self.runtime {
            ProtoRuntime::ProtobufPy => &constants.type_name,
            ProtoRuntime::Google => &constants.full_name,
        };
        self.descriptor
            .bind(py)
            .getattr(attribute)?
            .cast_into()
            .map_err(Into::into)
    }

    /// Resolves one path element to a field of this adapter's message.
    ///
    /// Matching is by field number, which is what the path is authoritative
    /// about; the name is only a fallback for paths that omit the number.
    pub(crate) fn find_field<'py>(
        &self,
        py: Python<'py>,
        element: &Bound<'py, PyAny>,
        constants: &Constants,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let number: i32 = element
            .getattr(&constants.field_number)?
            .extract()
            .unwrap_or(0);
        let name = element.getattr(&constants.field_name)?;

        match self.runtime {
            ProtoRuntime::ProtobufPy => {
                let fields = self.descriptor.bind(py).getattr(&constants.fields)?;
                if number != 0 {
                    for field in fields.try_iter()? {
                        let field = field?;
                        let candidate: i32 = field
                            .getattr(&constants.proto)?
                            .getattr(&constants.number)?
                            .extract()?;
                        if candidate == number {
                            return Ok(Some(field));
                        }
                    }
                }
                if !name.is_none() {
                    for field in fields.try_iter()? {
                        let field = field?;
                        // Compared as Python strings; extracting either side
                        // would allocate a Rust String per candidate.
                        if field.getattr(&constants.name)?.eq(&name)? {
                            return Ok(Some(field));
                        }
                    }
                }
                Ok(None)
            }
            ProtoRuntime::Google => {
                let descriptor = self.descriptor.bind(py);
                if number != 0
                    && let Ok(found) = descriptor
                        .getattr(&constants.fields_by_number)?
                        .get_item(number)
                {
                    return Ok(Some(found));
                }
                if !name.is_none()
                    && let Ok(found) = descriptor
                        .getattr(&constants.fields_by_name)?
                        .get_item(&name)
                {
                    return Ok(Some(found));
                }
                Ok(None)
            }
        }
    }
}

/// Where a message class stores one field.
pub(crate) struct FieldInfo {
    pub(crate) number: u32,
    /// The attribute holding the value: the proto name for google.protobuf,
    /// the Python-local name for protobuf-py.
    attr: Py<PyString>,
    /// For a protobuf-py field in a oneof, the oneof's attribute and the
    /// field's proto name, which is how the `Oneof` value identifies the
    /// set field.
    oneof: Option<(Py<PyString>, Py<PyString>)>,
}

impl FieldInfo {
    /// The field's value, `None` when it is not set and the runtime has no
    /// default object for it.
    pub(crate) fn fetch<'py>(
        &self,
        message: &Bound<'py, PyAny>,
        constants: &Constants,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some((oneof, name)) = &self.oneof {
            let selected = message.getattr(oneof)?;
            if selected.is_none() {
                return Ok(None);
            }
            if !selected.getattr(&constants.field)?.eq(name)? {
                return Ok(None);
            }
            return Ok(Some(selected.getattr(&constants.value)?));
        }
        let value = message.getattr(&self.attr)?;
        Ok((!value.is_none()).then_some(value))
    }
}

/// A callback that registers one descriptor file, given the file object
/// and its serialized `FileDescriptorProto`.
pub(crate) type AddFile<'a> =
    dyn FnMut(&Bound<'_, PyAny>, &Bound<'_, PyBytes>) -> PyResult<()> + 'a;

/// The entries of a map field, in the runtime's order.
pub(crate) enum MapEntries<'py> {
    /// A protobuf-py `dict`.
    Dict(BoundDictIterator<'py>),
    /// The `items()` of a google.protobuf map container.
    Items(Bound<'py, PyIterator>),
}

impl<'py> Iterator for MapEntries<'py> {
    type Item = PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Dict(entries) => entries.next().map(Ok),
            Self::Items(entries) => entries.next().map(|entry| entry?.extract()),
        }
    }
}

impl ProtoRuntime {
    /// The serialized message.
    pub(crate) fn serialize<'py>(
        self,
        message: &Bound<'py, PyAny>,
        constants: &Constants,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let method = match self {
            Self::ProtobufPy => &constants.to_binary,
            Self::Google => &constants.serialize_to_string,
        };
        message
            .call_method0(method)?
            .cast_into()
            .map_err(Into::into)
    }

    /// Registers a descriptor file and, before it, its imports, by passing
    /// each file and its serialized descriptor to `add`.
    pub(crate) fn collect_files(
        self,
        file: &Bound<'_, PyAny>,
        constants: &Constants,
        registered: &mut HashSet<String>,
        add: &mut AddFile<'_>,
    ) -> PyResult<()> {
        let name_attr = file.getattr(&constants.name)?;
        let name = name_attr.cast::<PyString>()?.to_str()?;
        if registered.contains(name) {
            return Ok(());
        }
        let name = name.to_owned();
        for dependency in file.getattr(&constants.dependencies)?.try_iter()? {
            self.collect_files(&dependency?, constants, registered, add)?;
        }
        let bytes: Bound<'_, PyBytes> = match self {
            Self::ProtobufPy => file
                .getattr(&constants.proto)?
                .call_method0(&constants.to_binary)?
                .cast_into()?,
            // `serialized_pb` is already a serialized FileDescriptorProto
            Self::Google => file.getattr(&constants.serialized_pb)?.cast_into()?,
        };
        add(file, &bytes)?;
        registered.insert(name);
        Ok(())
    }

    /// A descriptor's options as a protobuf-py message.
    pub(crate) fn options<'py>(
        self,
        py: Python<'py>,
        descriptor: &Bound<'py, PyAny>,
        reparse_type: &Py<PyType>,
        constants: &Constants,
    ) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::ProtobufPy => descriptor
                .getattr(&constants.proto)?
                .getattr(&constants.options),
            Self::Google => {
                // We roundtrip the Google options into a protobuf-py message to
                // match the return type of our public API.
                let serialized = descriptor
                    .call_method0(&constants.get_options)?
                    .call_method0(&constants.serialize_to_string)?;
                reparse_type
                    .bind(py)
                    .call_method1(&constants.from_binary, (serialized,))
            }
        }
    }

    /// Returns how to read each field of the message type that
    /// `descriptor` describes.
    pub(crate) fn fields(
        self,
        py: Python<'_>,
        descriptor: &Bound<'_, PyAny>,
        constants: &Constants,
    ) -> PyResult<Vec<FieldInfo>> {
        let mut fields = Vec::new();
        for field in descriptor.getattr(&constants.fields)?.try_iter()? {
            fields.push(self.field_info(py, &field?, constants)?);
        }
        Ok(fields)
    }

    /// Where a message class stores the field `field` describes.
    pub(crate) fn field_info(
        self,
        py: Python<'_>,
        field: &Bound<'_, PyAny>,
        constants: &Constants,
    ) -> PyResult<FieldInfo> {
        let intern = |name: Bound<'_, PyAny>| -> PyResult<Py<PyString>> {
            let name = name.cast_into::<PyString>()?;
            Ok(PyString::intern(py, name.to_str()?).unbind())
        };
        let number = field.getattr(&constants.number)?.extract()?;
        Ok(match self {
            Self::Google => FieldInfo {
                number,
                attr: intern(field.getattr(&constants.name)?)?,
                oneof: None,
            },
            Self::ProtobufPy => {
                // Only a singular value has a `oneof`, and it is `None` for
                // a proto3 `optional` field, whose synthetic oneof
                // protobuf-py does not surface: such a field is stored like
                // any other.
                let oneof = match field.getattr(&constants.value)?.getattr(&constants.oneof) {
                    Ok(oneof) if !oneof.is_none() => Some((
                        intern(oneof.getattr(&constants.local_name)?)?,
                        intern(field.getattr(&constants.name)?)?,
                    )),
                    _ => None,
                };
                FieldInfo {
                    number,
                    attr: intern(field.getattr(&constants.local_name)?)?,
                    oneof,
                }
            }
        })
    }

    /// Whether a field that tracks presence is set.
    pub(crate) fn is_set(
        self,
        message: &Bound<'_, PyAny>,
        field: &FieldInfo,
        kind: Kind,
        fetched: impl FnOnce() -> PyResult<bool>,
        constants: &Constants,
    ) -> PyResult<bool> {
        match self {
            // google.protobuf tracks it: `HasField`.
            Self::Google => message
                .call_method1(&constants.has_field, (&field.attr,))?
                .extract(),
            // protobuf-py: a oneof member is set when the oneof selects it,
            // a message field when it is not `None`, and any other field
            // when it is in the message's `_present` set.
            Self::ProtobufPy
                if field.oneof.is_some() || matches!(kind, Kind::Singular(Singular::Message)) =>
            {
                fetched()
            }
            Self::ProtobufPy => message.getattr(&constants.present)?.contains(field.number),
        }
    }

    /// Appends the elements of a repeated field to `items`. The field is a
    /// `list` for protobuf-py and a repeated container for
    /// google.protobuf, which creates a new element object on every access.
    pub(crate) fn list_items(
        self,
        object: &Bound<'_, PyAny>,
        items: &mut Vec<Py<PyAny>>,
    ) -> PyResult<()> {
        match self {
            Self::ProtobufPy => items.extend(object.cast::<PyList>()?.iter().map(Bound::unbind)),
            Self::Google => {
                for item in object.try_iter()? {
                    items.push(item?.unbind());
                }
            }
        }
        Ok(())
    }

    /// The entries of a map field: a `dict` for protobuf-py, a map container
    /// for google.protobuf.
    pub(crate) fn map_entries<'py>(
        self,
        object: &Bound<'py, PyAny>,
        constants: &Constants,
    ) -> PyResult<MapEntries<'py>> {
        Ok(match self {
            Self::ProtobufPy => MapEntries::Dict(object.cast::<PyDict>()?.iter()),
            Self::Google => MapEntries::Items(object.call_method0(&constants.items)?.try_iter()?),
        })
    }
}
