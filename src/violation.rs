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

//! The `Violation` type, re-exported by `protovalidate` at its original path;
//! the construction of violation lists from the engine's serialized output;
//! and the path-walking that recovers `field_value` and `rule_value` from a
//! violation's field and rule paths on first access.

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyBytes, PyDict, PyList, PyString};

use crate::constants::{Constants, Imports};
use crate::hints::{PbFieldPath, ViolationProto};
use crate::proto::ProtoAdapter;

/// The recovered values, resolved together because the rule walk needs the
/// leaf field found by the field walk.
struct Values {
    field: Py<PyAny>,
    rule: Py<PyAny>,
}

/// Where an engine-produced violation came from, kept for lazy resolution
/// of a violation field value.
struct Origin {
    message: Py<PyAny>,
    adapter: ProtoAdapter,
    imports: Arc<Imports>,
}

/// A singular rule violation.
#[pyclass(module = "protovalidate._protovalidate", frozen)]
pub struct Violation {
    /// The `buf.validate.Violation` form of this violation.
    proto: Py<PyAny>,
    /// `None` for hand-built violations, which have nothing to resolve from.
    origin: Option<Origin>,
    values: PyOnceLock<Values>,
}

impl Violation {
    /// Wraps a violation the engine produced, deferring value resolution.
    pub(crate) fn deferred(
        proto: Py<PyAny>,
        message: Py<PyAny>,
        adapter: ProtoAdapter,
        imports: Arc<Imports>,
    ) -> Self {
        Self {
            proto,
            origin: Some(Origin {
                message,
                adapter,
                imports,
            }),
            values: PyOnceLock::new(),
        }
    }

    /// Resolves both values on first use.
    fn values(&self, py: Python<'_>) -> PyResult<&Values> {
        self.values.get_or_try_init(py, || match &self.origin {
            Some(origin) => {
                let constants = Constants::get(py);
                let (field, rule) = resolve_values(
                    py,
                    self.proto.bind(py),
                    origin.message.bind(py),
                    &origin.adapter,
                    &constants,
                    &origin.imports,
                )?;
                Ok(Values { field, rule })
            }
            None => Ok(Values {
                field: py.None(),
                rule: py.None(),
            }),
        })
    }
}

#[pymethods]
impl Violation {
    /// Create a new violation.
    #[new]
    #[pyo3(signature = (
        *,
        field_value = None,
        rule_value = None,
        field = None,
        rule = None,
        rule_id = None,
        message = None,
        for_key = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn py_new(
        py: Python<'_>,
        field_value: Option<Py<PyAny>>,
        rule_value: Option<Py<PyAny>>,
        field: Option<PbFieldPath<'_, '_>>,
        rule: Option<PbFieldPath<'_, '_>>,
        rule_id: Option<Bound<'_, PyString>>,
        message: Option<Bound<'_, PyString>>,
        for_key: bool,
    ) -> PyResult<Self> {
        let constants = Constants::get(py);
        let imports = Imports::resolve(py)?;
        let kwargs = PyDict::new(py);
        kwargs.set_item(&constants.for_key, for_key)?;
        if let Some(rule_id) = rule_id {
            kwargs.set_item(&constants.rule_id, rule_id)?;
        }
        if let Some(message) = message {
            kwargs.set_item(&constants.message, message)?;
        }
        if let Some(field) = field {
            kwargs.set_item(&constants.field, field.0)?;
        }
        if let Some(rule) = rule {
            kwargs.set_item(&constants.rule, rule.0)?;
        }
        let proto = imports
            .types
            .violation
            .bind(py)
            .call((), Some(&kwargs))?
            .unbind();
        let values = PyOnceLock::new();
        let _ = values.set(
            py,
            Values {
                field: field_value.unwrap_or_else(|| py.None()),
                rule: rule_value.unwrap_or_else(|| py.None()),
            },
        );
        Ok(Self {
            proto,
            origin: None,
            values,
        })
    }

    /// The `buf.validate.Violation` form of this violation.
    #[getter]
    fn proto<'py>(&self, py: Python<'py>) -> ViolationProto<'py> {
        ViolationProto(self.proto.bind(py).clone())
    }

    /// The value of the field that violated the rule.
    #[getter]
    fn field_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.values(py)?.field.clone_ref(py))
    }

    /// The value of the rule that was violated.
    #[getter]
    fn rule_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.values(py)?.rule.clone_ref(py))
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!("Violation({})", self.proto.bind(py).repr()?))
    }
}

/// Recovers `(field_value, rule_value)` for one violation from its paths.
fn resolve_values(
    py: Python<'_>,
    proto: &Bound<'_, PyAny>,
    message: &Bound<'_, PyAny>,
    adapter: &ProtoAdapter,
    constants: &Constants,
    imports: &Imports,
) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
    let field = proto.getattr(&constants.field)?;
    let rule = proto.getattr(&constants.rule)?;
    let for_key: bool = proto.getattr(&constants.for_key)?.extract()?;

    let (field_value, leaf_field) = walk_field_path(py, message, &field, for_key, constants)?;
    let rule_value =
        resolve_rule_value(py, leaf_field.as_ref(), &rule, adapter, constants, imports)?;
    Ok((field_value, rule_value))
}

/// One hop of a path walk: resolves the element's field on `current` and
/// reads its value, returning both.
///
/// Re-resolving per hop keeps each step's descriptor at hand and makes
/// "not a message" fall out of resolution failing. `None` when the element
/// does not resolve.
fn step_element<'py>(
    py: Python<'py>,
    current: &Bound<'py, PyAny>,
    element: &Bound<'py, PyAny>,
    constants: &Constants,
) -> PyResult<Option<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
    let Ok(hop) = ProtoAdapter::resolve(current, constants) else {
        return Ok(None);
    };
    let Some(field) = hop.find_field(py, element, constants)? else {
        return Ok(None);
    };
    let Ok(value) = hop.runtime.read_field(current, &field, constants) else {
        return Ok(None);
    };
    Ok(Some((value, field)))
}

/// Walks a field path over the validated message.
///
/// Returns the value and the field the path ended on; the latter owns the rules
/// the rule path is relative to. Both values are best effort: a path can
/// legitimately fail to resolve -- an extension the registry does not know, or a
/// field only present in wire data -- and an informational value is not worth
/// raising over, so unresolvable paths yield `None`.
fn walk_field_path<'py>(
    py: Python<'py>,
    message: &Bound<'py, PyAny>,
    path: &Bound<'py, PyAny>,
    for_key: bool,
    constants: &Constants,
) -> PyResult<(Py<PyAny>, Option<Bound<'py, PyAny>>)> {
    if path.is_none() {
        return Ok((py.None(), None));
    }
    let elements = path.getattr(&constants.elements)?;
    let count = elements.len().unwrap_or(0);
    if count == 0 {
        return Ok((py.None(), None));
    }

    let mut current = message.clone();
    let mut leaf: Option<Bound<'py, PyAny>> = None;
    for (index, element) in elements.try_iter()?.enumerate() {
        let element = element?;
        let Some((value, field)) = step_element(py, &current, &element, constants)? else {
            return Ok((py.None(), None));
        };
        leaf = Some(field);
        current = value;

        let subscript = element.getattr(&constants.subscript)?;
        if !subscript.is_none() {
            let key = subscript.getattr(&constants.value)?;
            // A violation flagged `for_key` is about the map key itself, so the
            // subscript is the value in question rather than a step towards it.
            if for_key && index + 1 == count {
                return Ok((key.unbind(), leaf));
            }
            let Ok(value) = current.get_item(&key) else {
                return Ok((py.None(), None));
            };
            current = value;
        }
    }
    Ok((current.unbind(), leaf))
}

/// Walks a rule path over the rules that produced the violation.
///
/// Field-level rules are rooted at the leaf field's `(buf.validate.field)`
/// options; a violation with no field path came from a message-level rule and is
/// rooted at `(buf.validate.message)`. The rules are protobuf-py messages
/// whichever flavour was validated, so the walk from there is uniform.
fn resolve_rule_value<'py>(
    py: Python<'py>,
    leaf_field: Option<&Bound<'py, PyAny>>,
    path: &Bound<'py, PyAny>,
    adapter: &ProtoAdapter,
    constants: &Constants,
    imports: &Imports,
) -> PyResult<Py<PyAny>> {
    if path.is_none() {
        return Ok(py.None());
    }
    let elements = path.getattr(&constants.elements)?;
    if elements.len().unwrap_or(0) == 0 {
        return Ok(py.None());
    }

    let root = if let Some(field) = leaf_field {
        rules_of(py, field, adapter, constants, imports, RuleScope::Field)?
    } else {
        rules_of(
            py,
            adapter.descriptor(py),
            adapter,
            constants,
            imports,
            RuleScope::Message,
        )?
    };
    let Some(mut current) = root else {
        return Ok(py.None());
    };

    for element in elements.try_iter()? {
        let element = element?;
        // The rules are messages of whichever runtime parsed the options;
        // per-hop resolution handles them uniformly.
        let Some((value, _)) = step_element(py, &current, &element, constants)? else {
            return Ok(py.None());
        };
        current = value;

        let subscript = element.getattr(&constants.subscript)?;
        if !subscript.is_none() {
            let Ok(value) = current.get_item(subscript.getattr(&constants.value)?) else {
                return Ok(py.None());
            };
            current = value;
        }
    }
    Ok(current.unbind())
}

/// Whether rules are being read off a field or a message type.
#[derive(Clone, Copy)]
enum RuleScope {
    Field,
    Message,
}

/// The `buf.validate` rules attached to a descriptor, if any.
fn rules_of<'py>(
    py: Python<'py>,
    descriptor: &Bound<'py, PyAny>,
    adapter: &ProtoAdapter,
    constants: &Constants,
    imports: &Imports,
    scope: RuleScope,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let reparse_type = match scope {
        RuleScope::Field => &imports.types.field_options,
        RuleScope::Message => &imports.types.message_options,
    };
    let options = adapter
        .runtime
        .options(py, descriptor, reparse_type, constants)?;
    let extension = match scope {
        RuleScope::Field => &imports.extensions.field,
        RuleScope::Message => &imports.extensions.message,
    };
    Ok(options.get_item(extension.bind(py)).ok())
}

/// Turns the serialized `buf.validate.Violations` from C++ into wrappers.
///
/// No value resolution happens here: each wrapper keeps the proto the C++ side
/// produced together with the message it came from, and `field_value` and
/// `rule_value` are recovered from the paths on first access. Raising a
/// `ValidationError` or reading `rule_id` never pays for path walking.
pub fn build_violations<'py>(
    py: Python<'py>,
    serialized: &Bound<'py, PyBytes>,
    message: &Bound<'py, PyAny>,
    adapter: &ProtoAdapter,
    constants: &Constants,
    imports: &Arc<Imports>,
) -> PyResult<Bound<'py, PyList>> {
    let parsed = imports
        .types
        .violations
        .bind(py)
        .call_method1(&constants.from_binary, (serialized,))?;
    // protobuf-py repeated fields are guaranteed to be plain lists.
    let violations = parsed
        .getattr(&constants.violations)?
        .cast_into::<PyList>()?;
    PyList::new(
        py,
        violations.iter().map(|violation| {
            Violation::deferred(
                violation.unbind(),
                message.clone().unbind(),
                adapter.clone_ref(py),
                Arc::clone(imports),
            )
        }),
    )
}
