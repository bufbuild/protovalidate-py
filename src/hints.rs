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

//! No-cost wrappers whose job is carrying Python type hints for stub generation.
//! `PyO3` uses the static type information when generating stubs with introspection.

use std::convert::Infallible;

use pyo3::inspect::PyStaticExpr;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::{Borrowed, type_hint_identifier, type_hint_subscript, type_hint_union};

use crate::violation::Violation;

const VALIDATE_PB: &str = "protovalidate._gen.buf.validate.validate_pb";

/// A protobuf message of either runtime, hinted as the union of both.
#[derive(Clone, Copy)]
pub(crate) struct PbMessage<'a, 'py>(pub(crate) Borrowed<'a, 'py, PyAny>);

impl<'a, 'py> FromPyObject<'a, 'py> for PbMessage<'a, 'py> {
    type Error = Infallible;

    const INPUT_TYPE: PyStaticExpr = type_hint_union!(
        type_hint_identifier!("protobuf", "Message"),
        type_hint_identifier!("google.protobuf.message", "Message")
    );

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        Ok(Self(obj))
    }
}

/// A protobuf-py `Registry`.
pub(crate) struct PbRegistry<'a, 'py>(pub(crate) Borrowed<'a, 'py, PyAny>);

impl<'a, 'py> FromPyObject<'a, 'py> for PbRegistry<'a, 'py> {
    type Error = Infallible;

    const INPUT_TYPE: PyStaticExpr = type_hint_identifier!("protobuf", "Registry");

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        Ok(Self(obj))
    }
}

/// A `buf.validate.FieldPath` message.
pub(crate) struct PbFieldPath<'a, 'py>(pub(crate) Borrowed<'a, 'py, PyAny>);

impl<'a, 'py> FromPyObject<'a, 'py> for PbFieldPath<'a, 'py> {
    type Error = Infallible;

    const INPUT_TYPE: PyStaticExpr = type_hint_identifier!(VALIDATE_PB, "FieldPath");

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        Ok(Self(obj))
    }
}

/// A list of [`Violation`] wrappers, hinted as `list[Violation]`.
pub(crate) struct ViolationList<'py>(pub(crate) Bound<'py, PyList>);

impl<'py> IntoPyObject<'py> for ViolationList<'py> {
    type Target = PyList;
    type Output = Bound<'py, PyList>;
    type Error = Infallible;

    const OUTPUT_TYPE: PyStaticExpr = type_hint_subscript!(
        type_hint_identifier!("builtins", "list"),
        <Violation as pyo3::PyTypeInfo>::TYPE_HINT
    );

    fn into_pyobject(self, _py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.0)
    }
}

/// A `buf.validate.Violation` message.
pub(crate) struct ViolationProto<'py>(pub(crate) Bound<'py, PyAny>);

impl<'py> IntoPyObject<'py> for ViolationProto<'py> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = Infallible;

    const OUTPUT_TYPE: PyStaticExpr = type_hint_identifier!(VALIDATE_PB, "Violation");

    fn into_pyobject(self, _py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.0)
    }
}
