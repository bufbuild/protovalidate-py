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

//! Python bindings for the `protovalidate` crate.

mod constants;
mod hints;
mod runtime;
mod view;
mod violation;

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use pyo3::exceptions::{PyException, PyValueError};
use pyo3::import_exception;
use pyo3::prelude::*;
use pyo3::sync::{PyOnceLock, RwLockExt};
use pyo3::types::{PyBytes, PyList, PyString};

use protovalidate::{DescriptorError, Error};

use constants::{Constants, Imports};
use hints::{PbMessage, PbRegistry, ViolationList};
use runtime::{ProtoAdapter, ProtoRuntime};
use view::{Ctx, PyRuntime, TypeSource};

/// The core validator for messages of one Python runtime.
type Core = protovalidate::Validator<PyRuntime>;

import_exception!(protovalidate._errors, ValidationError);
import_exception!(protovalidate._errors, CompilationError);
import_exception!(protovalidate._errors, EvaluationError);

/// Maps a failure of validation itself; `Error::Validation` is handled by
/// the caller, as it is a result rather than a failure.
fn to_py_err(error: Error<Box<PyErr>>) -> PyErr {
    match error {
        // The Python error raised while the message was being read, as it
        // was raised.
        Error::Read(error) => *error,
        Error::Compilation(message) => CompilationError::new_err(message),
        Error::Evaluation(message) => EvaluationError::new_err(message),
        Error::Argument(message) => PyValueError::new_err(message),
        // Neither a compilation nor an evaluation failure; a plain Exception
        // keeps it out of both buckets rather than mislabelling it.
        Error::Unexpected(message) => PyException::new_err(message),
        other => PyException::new_err(other.to_string()),
    }
}

fn descriptor_err(error: &DescriptorError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// Validate Protobuf messages against static rules.
///
/// Both protobuf-py messages and legacy google.protobuf messages are
/// accepted; either is validated in place, and only serialized if a custom
/// CEL rule needs a message, list or map as a value.
///
/// Each validator instance caches internal state generated from the static
/// rules, so reusing the same instance for multiple validations
/// significantly improves performance.
#[pyclass(module = "protovalidate._protovalidate", frozen)]
struct Validator {
    /// One engine per Python runtime. Compiled rules record how to read
    /// each message type's fields, and that differs between the two
    /// runtimes, so they cannot share an engine.
    engines: [PyOnceLock<Engine>; 2],
    /// The files from the constructor's `registry` argument that declare
    /// extensions. They are registered with each engine when it is created.
    preregistered: Vec<Py<PyAny>>,
    /// Interned strings, shared by every call site.
    constants: Constants,
    /// Python types and extensions.
    imports: Arc<Imports>,
}

/// The core validator and registration state for one Python runtime.
struct Engine {
    /// Adding descriptors needs exclusive access and happens only while
    /// warming up; a `RwLock` leaves the steady state unblocked.
    core: RwLock<Core>,
    /// Descriptor files already added to the pool, by name. Mutated together
    /// with the core, under both write locks; see `register`.
    registered: RwLock<HashSet<String>>,
    /// For protobuf-py, a `Registry` holding every registered file, used to
    /// look up message types by name. `None` for google.protobuf, whose
    /// types are looked up in the descriptor pool of the message being
    /// validated.
    registry: Option<Py<PyAny>>,
}

#[pymethods]
impl Validator {
    /// Create a new validator.
    ///
    /// Parameters:
    ///     registry: An optional Registry whose files declaring extensions are
    ///         registered up front, so predefined rules defined in files the
    ///         validated messages do not import are still found.
    #[new]
    #[pyo3(signature = (registry = None))]
    fn new(py: Python<'_>, registry: Option<PbRegistry<'_, '_>>) -> PyResult<Self> {
        let constants = Constants::get(py);
        let preregistered = match registry {
            Some(registry) => collect_registry(&registry.0, &constants)?,
            None => Vec::new(),
        };
        Ok(Self {
            engines: [PyOnceLock::new(), PyOnceLock::new()],
            preregistered,
            constants,
            imports: Arc::new(Imports::resolve(py)?),
        })
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
        let engine = self.engine(py, adapter.runtime)?;
        let file = adapter.descriptor(py).getattr(&self.constants.file)?;
        engine.register(py, adapter.runtime, &file, &self.constants)?;

        let type_name = adapter.type_name(py, &self.constants)?;
        let Some(serialized) = self.evaluate(
            py,
            engine,
            &adapter,
            type_name.to_str()?,
            &message.0,
            fail_fast,
        )?
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

    /// Returns the engine for `runtime`.
    fn engine(&self, py: Python<'_>, runtime: ProtoRuntime) -> PyResult<&Engine> {
        let slot = match runtime {
            ProtoRuntime::ProtobufPy => 0,
            ProtoRuntime::Google => 1,
        };
        self.engines[slot].get_or_try_init(py, || {
            Engine::new(
                py,
                runtime,
                &self.preregistered,
                &self.constants,
                &self.imports,
            )
        })
    }

    /// Validates the message in place, returning serialized violations, or
    /// `None` when the message is valid.
    ///
    /// Reading the message calls into Python, so the interpreter stays
    /// attached throughout; the core lock is taken with the
    /// interpreter-aware variant so a registration waiting on it cannot
    /// deadlock with a validation that Python has preempted.
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        engine: &Engine,
        adapter: &ProtoAdapter,
        type_name: &str,
        message: &Bound<'py, PyAny>,
        fail_fast: bool,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        let core = engine.core.read_py_attached(py).unwrap();
        let registry = engine.registry.as_ref().map(|registry| registry.bind(py));
        let source = match registry {
            Some(registry) => TypeSource::Registry(registry),
            None => TypeSource::Descriptor(adapter.descriptor(py)),
        };
        let ctx = Ctx::new(py, adapter.runtime, &self.constants, message, source);
        match core.validate_message(type_name, &ctx, fail_fast) {
            Ok(()) => Ok(None),
            Err(Error::Validation(error)) => Ok(Some(PyBytes::new(py, error.violations()))),
            Err(error) => Err(to_py_err(error)),
        }
    }
}

impl Engine {
    /// Creates the engine for `runtime` and registers the protobuf-py
    /// `DescFile`s in `preregistered` with it.
    fn new(
        py: Python<'_>,
        runtime: ProtoRuntime,
        preregistered: &[Py<PyAny>],
        constants: &Constants,
        imports: &Imports,
    ) -> PyResult<Self> {
        let registry = match runtime {
            ProtoRuntime::ProtobufPy => Some(imports.types.registry.bind(py).call0()?.unbind()),
            ProtoRuntime::Google => None,
        };
        let engine = Self {
            core: RwLock::new(Core::new()),
            registered: RwLock::new(HashSet::new()),
            registry,
        };
        for file in preregistered {
            engine.register(py, ProtoRuntime::ProtobufPy, file.bind(py), constants)?;
        }
        Ok(engine)
    }

    /// Registers `file`, a descriptor file of `runtime`, together with its
    /// imports. Files already registered are skipped.
    fn register(
        &self,
        py: Python<'_>,
        runtime: ProtoRuntime,
        file: &Bound<'_, PyAny>,
        constants: &Constants,
    ) -> PyResult<()> {
        let name_attr = file.getattr(&constants.name)?;
        let name = name_attr.cast::<PyString>()?.to_str()?;
        if self.registered.read_py_attached(py).unwrap().contains(name) {
            return Ok(());
        }
        // Registration is rare (once per file), so holding the write locks
        // across the collection walk costs little.
        let mut registered = self.registered.write_py_attached(py).unwrap();
        let mut core = self.core.write_py_attached(py).unwrap();
        let registry = self.registry.as_ref().map(|registry| registry.bind(py));
        runtime.collect_files(file, constants, &mut registered, &mut |file, bytes| {
            core.add_file_descriptor_bytes(bytes.as_bytes())
                .map_err(|error| descriptor_err(&error))?;
            if let Some(registry) = registry {
                registry.call_method1(&constants.add, (file,))?;
            }
            Ok(())
        })
    }
}

/// Returns the `DescFile`s in `registry` that declare extensions.
///
/// Predefined-rule extensions may live in files nothing being validated
/// imports, in which case the lazy walk over message imports would never reach
/// them. Only files declaring extensions can carry such rules, so the rest of
/// the registry is left alone.
fn collect_registry(
    registry: &Bound<'_, PyAny>,
    constants: &Constants,
) -> PyResult<Vec<Py<PyAny>>> {
    let mut files = Vec::new();
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
        files.push(file.unbind());
    }
    Ok(files)
}

/// The native protovalidate engine.
#[pymodule(gil_used = false)]
mod _protovalidate {
    #[pymodule_export]
    use super::Validator;
    #[pymodule_export]
    use super::violation::Violation;
}
