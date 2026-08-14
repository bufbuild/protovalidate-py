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

//! The adapter over the two Python Protobuf runtimes. Everything that must be
//! asked differently of protobuf-py and google.protobuf -- descriptors, type
//! names, serialization, field access -- goes through [`ProtoAdapter`], so the
//! rest of the crate is runtime-agnostic.

use std::collections::HashSet;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString, PyType};

use crate::constants::Constants;

/// Which Python protobuf runtime a message comes from.
#[derive(Clone, Copy)]
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

impl ProtoRuntime {
    /// The serialized message.
    pub(crate) fn payload<'py>(
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

    /// Adds a descriptor file and its imports to the engine, imports first.
    ///
    /// The shim adds descriptors with `BuildFile`, which resolves dependencies
    /// eagerly, so a file's imports have to be added before the file itself;
    /// the recursion order preserves that. `registered` doubles as the walk's
    /// visited set; a file is recorded only once `add` has accepted it.
    pub(crate) fn collect_files(
        self,
        file: &Bound<'_, PyAny>,
        constants: &Constants,
        registered: &mut HashSet<String>,
        add: &mut dyn FnMut(&Bound<'_, PyBytes>) -> PyResult<()>,
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
        add(&bytes)?;
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

    /// Reads a field's value off a message.
    pub(crate) fn read_field<'py>(
        self,
        message: &Bound<'py, PyAny>,
        field: &Bound<'py, PyAny>,
        constants: &Constants,
    ) -> PyResult<Bound<'py, PyAny>> {
        match self {
            Self::ProtobufPy => message.get_item(field),
            Self::Google => {
                let name = field.getattr(&constants.name)?;
                message.getattr(name.cast_into::<PyString>()?)
            }
        }
    }
}
