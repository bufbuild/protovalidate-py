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

//! Interned strings, and the Python types the bindings reach for.

use std::ops::Deref;
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyString, PyType};

/// Python types the bindings construct or parse with.
pub(crate) struct Types {
    /// The class `protovalidate._gen.buf.validate.validate_pb.Violations`.
    pub(crate) violations: Py<PyType>,
    /// The class `protovalidate._gen.buf.validate.validate_pb.Violation`.
    pub(crate) violation: Py<PyType>,
    /// The class `protobuf.wkt.FieldOptions`.
    pub(crate) field_options: Py<PyType>,
    /// The class `protobuf.wkt.MessageOptions`.
    pub(crate) message_options: Py<PyType>,
}

/// Extension objects used to read rules out of descriptor options.
pub(crate) struct Extensions {
    /// The extension `buf.validate.field`.
    pub(crate) field: Py<PyAny>,
    /// The extension `buf.validate.message`.
    pub(crate) message: Py<PyAny>,
}

pub(crate) struct ConstantsInner {
    /// The string `DESCRIPTOR`.
    pub(crate) descriptor_upper: Py<PyString>,
    /// The string `GetOptions`.
    pub(crate) get_options: Py<PyString>,
    /// The string `SerializeToString`.
    pub(crate) serialize_to_string: Py<PyString>,
    /// The string `dependencies`.
    pub(crate) dependencies: Py<PyString>,
    /// The string `desc`.
    pub(crate) desc: Py<PyString>,
    /// The string `elements`.
    pub(crate) elements: Py<PyString>,
    /// The string `extensions`.
    pub(crate) extensions: Py<PyString>,
    /// The string `field`.
    pub(crate) field: Py<PyString>,
    /// The string `field_name`.
    pub(crate) field_name: Py<PyString>,
    /// The string `field_number`.
    pub(crate) field_number: Py<PyString>,
    /// The string `fields`.
    pub(crate) fields: Py<PyString>,
    /// The string `fields_by_name`.
    pub(crate) fields_by_name: Py<PyString>,
    /// The string `fields_by_number`.
    pub(crate) fields_by_number: Py<PyString>,
    /// The string `file`.
    pub(crate) file: Py<PyString>,
    /// The string `for_key`.
    pub(crate) for_key: Py<PyString>,
    /// The string `from_binary`.
    pub(crate) from_binary: Py<PyString>,
    /// The string `full_name`.
    pub(crate) full_name: Py<PyString>,
    /// The string `message`.
    pub(crate) message: Py<PyString>,
    /// The string `name`.
    pub(crate) name: Py<PyString>,
    /// The string `number`.
    pub(crate) number: Py<PyString>,
    /// The string `options`.
    pub(crate) options: Py<PyString>,
    /// The string `proto`.
    pub(crate) proto: Py<PyString>,
    /// The string `rule`.
    pub(crate) rule: Py<PyString>,
    /// The string `rule_id`.
    pub(crate) rule_id: Py<PyString>,
    /// The string `serialized_pb`.
    pub(crate) serialized_pb: Py<PyString>,
    /// The string `subscript`.
    pub(crate) subscript: Py<PyString>,
    /// The string `to_binary`.
    pub(crate) to_binary: Py<PyString>,
    /// The string `type_name`.
    pub(crate) type_name: Py<PyString>,
    /// The string `value`.
    pub(crate) value: Py<PyString>,
    /// The string `violations`.
    pub(crate) violations: Py<PyString>,
}

/// Cheaply cloneable handle to the interned objects.
#[derive(Clone)]
pub(crate) struct Constants {
    inner: Arc<ConstantsInner>,
}

/// Static constants that can be cached per process.
impl Constants {
    pub(crate) fn get(py: Python<'_>) -> Self {
        static INSTANCE: PyOnceLock<Constants> = PyOnceLock::new();

        INSTANCE.get_or_init(py, || Self::new(py)).clone()
    }

    fn new(py: Python<'_>) -> Self {
        Self {
            inner: Arc::new(ConstantsInner {
                descriptor_upper: PyString::intern(py, "DESCRIPTOR").unbind(),
                get_options: PyString::intern(py, "GetOptions").unbind(),
                serialize_to_string: PyString::intern(py, "SerializeToString").unbind(),
                dependencies: PyString::intern(py, "dependencies").unbind(),
                desc: PyString::intern(py, "desc").unbind(),
                elements: PyString::intern(py, "elements").unbind(),
                extensions: PyString::intern(py, "extensions").unbind(),
                field: PyString::intern(py, "field").unbind(),
                field_name: PyString::intern(py, "field_name").unbind(),
                field_number: PyString::intern(py, "field_number").unbind(),
                fields: PyString::intern(py, "fields").unbind(),
                fields_by_name: PyString::intern(py, "fields_by_name").unbind(),
                fields_by_number: PyString::intern(py, "fields_by_number").unbind(),
                file: PyString::intern(py, "file").unbind(),
                for_key: PyString::intern(py, "for_key").unbind(),
                from_binary: PyString::intern(py, "from_binary").unbind(),
                full_name: PyString::intern(py, "full_name").unbind(),
                message: PyString::intern(py, "message").unbind(),
                name: PyString::intern(py, "name").unbind(),
                number: PyString::intern(py, "number").unbind(),
                options: PyString::intern(py, "options").unbind(),
                proto: PyString::intern(py, "proto").unbind(),
                rule: PyString::intern(py, "rule").unbind(),
                rule_id: PyString::intern(py, "rule_id").unbind(),
                serialized_pb: PyString::intern(py, "serialized_pb").unbind(),
                subscript: PyString::intern(py, "subscript").unbind(),
                to_binary: PyString::intern(py, "to_binary").unbind(),
                type_name: PyString::intern(py, "type_name").unbind(),
                value: PyString::intern(py, "value").unbind(),
                violations: PyString::intern(py, "violations").unbind(),
            }),
        }
    }
}

/// Python types and extension objects, resolved on demand.
pub(crate) struct Imports {
    pub(crate) types: Types,
    pub(crate) extensions: Extensions,
}

impl Imports {
    pub(crate) fn resolve(py: Python<'_>) -> PyResult<Self> {
        let validate = py.import("protovalidate._gen.buf.validate.validate_pb")?;
        let wkt = py.import("protobuf.wkt")?;
        Ok(Self {
            types: Types {
                violations: validate
                    .getattr("Violations")?
                    .cast_into::<PyType>()?
                    .unbind(),
                violation: validate
                    .getattr("Violation")?
                    .cast_into::<PyType>()?
                    .unbind(),
                field_options: wkt.getattr("FieldOptions")?.cast_into::<PyType>()?.unbind(),
                message_options: wkt
                    .getattr("MessageOptions")?
                    .cast_into::<PyType>()?
                    .unbind(),
            },
            extensions: Extensions {
                field: validate.getattr("ext_field")?.unbind(),
                message: validate.getattr("ext_message")?.unbind(),
            },
        })
    }
}

impl Deref for Constants {
    type Target = ConstantsInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
