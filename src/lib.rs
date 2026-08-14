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

//! Python bindings for protovalidate-cc.

mod constants;
mod hints;
mod proto;
mod violation;

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use pyo3::exceptions::{PyException, PyRuntimeError, PyValueError};
use pyo3::import_exception;
use pyo3::prelude::*;
use pyo3::sync::RwLockExt;
use pyo3::types::{PyBytes, PyList, PyString};

use protovalidate::{Engine, PvError};

use constants::{Constants, Imports};
use hints::{PbMessage, PbRegistry, ViolationList};
use proto::{ProtoAdapter, ProtoRuntime};

import_exception!(protovalidate._errors, ValidationError);
import_exception!(protovalidate._errors, CompilationError);
import_exception!(protovalidate._errors, EvaluationError);

fn to_py_err(error: PvError) -> PyErr {
    match error {
        PvError::Compilation(message) => CompilationError::new_err(message),
        PvError::Evaluation(message) => EvaluationError::new_err(message),
        PvError::Argument(message) => PyValueError::new_err(message),
        // Neither a compilation nor an evaluation failure; a plain Exception
        // keeps it out of both buckets rather than mislabelling it.
        PvError::Unexpected(message) => PyException::new_err(message),
        PvError::Unknown(code, message) => {
            PyException::new_err(format!("unknown status {code}: {message}"))
        }
    }
}

/// Validate Protobuf messages against static rules.
///
/// Both protobuf-py messages and legacy google.protobuf messages are
/// accepted; either is handed to the native engine in serialized form.
///
/// Each validator instance caches internal state generated from the static
/// rules, so reusing the same instance for multiple validations
/// significantly improves performance.
#[pyclass(module = "protovalidate._protovalidate", frozen)]
struct Validator {
    /// Engine isn't thread-safe when adding descriptors. Since this only happens when
    /// warming up, we use a `RwLock` to allow the steady state to have no blocking.
    engine: RwLock<Engine>,
    /// Descriptor files already added to the pool, by name. Mutated together
    /// with the engine, under both write locks; see `register`.
    registered: RwLock<HashSet<String>>,
    /// Interned strings, shared by every call site.
    constants: Constants,
    /// Python types and extensions.
    imports: Arc<Imports>,
}

#[pymethods]
impl Validator {
    /// Create a new validator.
    ///
    /// Parameters:
    ///     registry: An optional Registry used to resolve custom
    ///         predefined-rule extensions. If omitted, only standard rules are applied.
    #[new]
    #[pyo3(signature = (registry = None))]
    fn new(py: Python<'_>, registry: Option<PbRegistry<'_, '_>>) -> PyResult<Self> {
        let engine = Engine::new().map_err(PyRuntimeError::new_err)?;
        let validator = Self {
            engine: RwLock::new(engine),
            registered: RwLock::new(HashSet::new()),
            constants: Constants::get(py),
            imports: Arc::new(Imports::resolve(py)?),
        };
        if let Some(registry) = registry {
            validator.add_registry(py, &registry.0)?;
        }
        Ok(validator)
    }

    /// Validate the given message against the static rules defined in the message's descriptor.
    ///
    /// Parameters:
    ///     message: The message to validate.
    ///     fail_fast: If true, validation will stop after the first iteration.
    ///
    /// Raises:
    ///     CompilationError: If the static rules could not be compiled.
    ///     EvaluationError: If a rule failed while being evaluated.
    ///     ValidationError: If the message is invalid. The violations raised as part of this error should
    ///         always be equal to the list of violations returned by `collect_violations`.
    #[allow(clippy::doc_markdown)] // A Python docstring.
    #[pyo3(signature = (message, *, fail_fast = false))]
    fn validate(
        &self,
        py: Python<'_>,
        message: PbMessage<'_, '_>,
        fail_fast: bool,
    ) -> PyResult<()> {
        let (adapter, violations) = self.collect(py, message, fail_fast)?;
        if violations.0.is_empty() {
            return Ok(());
        }
        let name = adapter
            .descriptor(py)
            .getattr(&self.constants.name)?
            .cast_into::<PyString>()?;
        Err(ValidationError::new_err((
            format!("invalid {}", name.to_str()?),
            violations.0.unbind(),
        )))
    }

    /// Validates the given message against the static rules defined in the message's descriptor.
    ///
    /// Compared to `validate`, `collect_violations` simply returns the violations as a list and puts
    /// the burden of raising an appropriate exception on the caller.
    ///
    /// The violations returned from this method should always be equal to the violations
    /// raised as part of the ValidationError in the call to `validate`.
    ///
    /// Parameters:
    ///     message: The message to validate.
    ///     fail_fast: If true, validation will stop after the first iteration.
    ///
    /// Returns:
    ///     A list of Violation objects that describe the validation errors.
    ///
    /// Raises:
    ///     CompilationError: If the static rules could not be compiled.
    ///     EvaluationError: If a rule failed while being evaluated.
    #[allow(clippy::doc_markdown)] // A Python docstring.
    #[pyo3(signature = (message, *, fail_fast = false))]
    fn collect_violations<'py>(
        &self,
        py: Python<'py>,
        message: PbMessage<'_, 'py>,
        fail_fast: bool,
    ) -> PyResult<ViolationList<'py>> {
        Ok(self.collect(py, message, fail_fast)?.1)
    }
}

impl Validator {
    /// Resolves a message and collects its violations.
    fn collect<'py>(
        &self,
        py: Python<'py>,
        message: PbMessage<'_, 'py>,
        fail_fast: bool,
    ) -> PyResult<(ProtoAdapter, ViolationList<'py>)> {
        let adapter = ProtoAdapter::resolve(&message.0, &self.constants)?;
        let file = adapter.descriptor(py).getattr(&self.constants.file)?;
        self.register(py, &file, &adapter)?;

        let type_name = adapter.type_name(py, &self.constants)?;
        let payload = adapter.runtime.payload(&message.0, &self.constants)?;

        let Some(serialized) =
            self.evaluate(py, type_name.to_str()?, payload.as_bytes(), fail_fast)?
        else {
            // `descriptor` keeps the adapter borrowed for `'py`, so hand the
            // caller a cheap reference clone rather than the local.
            return Ok((adapter.clone_ref(py), ViolationList(PyList::empty(py))));
        };
        let violations = violation::build_violations(
            py,
            &serialized,
            &message.0,
            &adapter,
            &self.constants,
            &self.imports,
        )
        .map(ViolationList)?;
        Ok((adapter.clone_ref(py), violations))
    }
    /// Registers every descriptor file a registry contributes.
    fn add_registry(&self, py: Python<'_>, registry: &Bound<'_, PyAny>) -> PyResult<()> {
        let mut registered = self.registered.write_py_attached(py).unwrap();
        let mut engine = self.engine.write_py_attached(py).unwrap();
        collect_registry(registry, &self.constants, &mut registered, &mut |bytes| {
            engine.add_file(bytes.as_bytes()).map_err(to_py_err)
        })
    }

    /// Registers a descriptor file and its imports, skipping known ones.
    fn register(
        &self,
        py: Python<'_>,
        file: &Bound<'_, PyAny>,
        adapter: &ProtoAdapter,
    ) -> PyResult<()> {
        let name_attr = file.getattr(&self.constants.name)?;
        let name = name_attr.cast::<PyString>()?.to_str()?;
        if self.is_registered(py, name) {
            return Ok(());
        }
        // Registration is rare (once per file), so holding the write locks
        // across the collection walk costs little.
        let mut registered = self.registered.write_py_attached(py).unwrap();
        let mut engine = self.engine.write_py_attached(py).unwrap();
        adapter
            .runtime
            .collect_files(file, &self.constants, &mut registered, &mut |bytes| {
                engine.add_file(bytes.as_bytes()).map_err(to_py_err)
            })
    }

    fn is_registered(&self, py: Python<'_>, name: &str) -> bool {
        self.registered.read_py_attached(py).unwrap().contains(name)
    }

    /// Runs validation over serialized bytes, returning serialized violations,
    /// or `None` when the message is valid.
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        type_name: &str,
        payload: &[u8],
        fail_fast: bool,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        let violations = py
            .detach(|| {
                let engine = self.engine.read().unwrap();
                engine.validate(type_name, payload, fail_fast)
            })
            .map_err(to_py_err)?;
        Ok(violations.map(|buffer| PyBytes::new(py, buffer.as_slice())))
    }
}

/// Collects every file in a registry that declares extensions.
///
/// Predefined-rule extensions may live in files nothing being validated
/// imports, in which case the lazy walk over message imports would never reach
/// them. Only files declaring extensions can carry such rules, so the rest of
/// the registry is left alone.
fn collect_registry(
    registry: &Bound<'_, PyAny>,
    constants: &Constants,
    registered: &mut HashSet<String>,
    add: &mut dyn FnMut(&Bound<'_, PyBytes>) -> PyResult<()>,
) -> PyResult<()> {
    for descriptor in registry.try_iter()? {
        let descriptor = descriptor?;
        let Ok(extensions) = descriptor.getattr(&constants.extensions) else {
            continue;
        };
        if extensions.len().unwrap_or(0) == 0 {
            continue;
        }
        // A DescFile has no `file` attribute and stands for itself; nested
        // extensions come from a message, which does.
        let file = descriptor
            .getattr(&constants.file)
            .unwrap_or_else(|_| descriptor.clone());
        ProtoRuntime::ProtobufPy.collect_files(&file, constants, registered, add)?;
    }
    Ok(())
}

/// The native protovalidate engine.
#[pymodule(gil_used = false)]
mod _protovalidate {
    #[pymodule_export]
    use super::Validator;
    #[pymodule_export]
    use super::violation::Violation;
}
