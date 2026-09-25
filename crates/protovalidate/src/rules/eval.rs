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

//! Walking a message with its validators.
//!
//! A [`Walk`] is one validation. The validators drive it, each reading its
//! part of the message through the runtime. A violation records the field
//! path the walk is at and the rule path compiled into the check. Anything
//! that ends the walk early, a read failing or the first violation when
//! failing fast, propagates as an [`Abort`] through `?`.

use std::cell::OnceCell;
use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::sync::{PoisonError, RwLock};

use buffa_descriptor::{DescriptorPool, MessageIndex};

use super::standard::{
    Check, Checks, Duration, EnumTest, ListTest, MapTest, Test, Timestamp, UniqueKey,
    has_duplicates, total_nanos,
};
use super::{
    CelPrograms, FieldKind, FieldValidator, ItemValidator, MessageOneof, MessageValidator,
    OneofRequired, ValidatorCache, ValueValidator,
};
use crate::Error;
use crate::cel::{self, Env, Frame, ScalarValue, This, Value};
use crate::descriptors::{self, Descriptors, Schema, wkt};
use crate::protobuf::{
    Field, Kind, List as _, Map as _, Message as _, Runtime, Scalar, Singular, Val,
};
use crate::validate::__buffa::oneof::field_path_element::Subscript;
use crate::validate::{FieldPath, FieldPathElement, Violation};

/// Why the walk stopped before the end of the message.
pub(crate) enum Abort<E> {
    Error(Error<E>),
    /// A violation was found while failing fast.
    FailFast,
}

impl<E> From<Error<E>> for Abort<E> {
    fn from(error: Error<E>) -> Self {
        Self::Error(error)
    }
}

/// How the CEL backend's errors map to the validator's.
impl<E> From<cel::Error> for Abort<E> {
    fn from(error: cel::Error) -> Self {
        Self::Error(match error {
            cel::Error::Runtime(message) => Error::Evaluation(message),
            cel::Error::Argument(message) => Error::Argument(message),
            cel::Error::Compilation(message) | cel::Error::Unexpected(message) => {
                Error::Unexpected(message)
            }
        })
    }
}

/// Converts the result of reading the message into the walk's result.
trait Read<T, E> {
    fn read(self) -> Result<T, Abort<E>>;
}

impl<T, E> Read<T, E> for Result<T, E> {
    fn read(self) -> Result<T, Abort<E>> {
        self.map_err(|error| Abort::Error(Error::Read(error)))
    }
}

/// One element of the walk's current field path.
struct PathElement<'a> {
    element: &'a FieldPathElement,
    subscript: Option<Subscript>,
}

/// The violations found so far, and where the walk is.
pub(crate) struct Violations<'a> {
    fail_fast: bool,
    /// The field path from the root to the value being validated.
    path: Vec<PathElement<'a>>,
    /// Whether the value being validated is a map key.
    for_key: bool,
    list: Vec<Violation>,
}

impl Violations<'_> {
    fn new(fail_fast: bool) -> Self {
        Self {
            fail_fast,
            path: Vec::new(),
            for_key: false,
            list: Vec::new(),
        }
    }

    /// Records a violation at the current path. When failing fast, the
    /// walk stops here.
    fn push<E>(
        &mut self,
        id: &str,
        message: &str,
        rule_path: &[FieldPathElement],
    ) -> Result<(), Abort<E>> {
        let field_path = |elements: Vec<FieldPathElement>| {
            (!elements.is_empty()).then(|| FieldPath {
                elements,
                ..Default::default()
            })
        };
        let field = self
            .path
            .iter()
            .map(|at| FieldPathElement {
                subscript: at.subscript.clone(),
                ..at.element.clone()
            })
            .collect();
        self.list.push(Violation {
            field: field_path(field).into(),
            rule: field_path(rule_path.to_vec()).into(),
            rule_id: Some(id.to_owned()),
            message: Some(message.to_owned()),
            for_key: self.for_key.then_some(true),
            ..Default::default()
        });
        if self.fail_fast {
            Err(Abort::FailFast)
        } else {
            Ok(())
        }
    }

    /// Sets the subscript of the container field the walk is at to the
    /// item being validated.
    fn subscript(&mut self, subscript: Option<Subscript>) {
        if let Some(at) = self.path.last_mut() {
            at.subscript = subscript;
        }
    }

    /// Sets the subscript of the walk's current container field in every
    /// violation recorded since index `recorded`. The subscript is only
    /// computed if there is such a violation.
    fn subscript_since(&mut self, recorded: usize, subscript: impl FnOnce() -> Option<Subscript>) {
        let (Some(depth), Some(violations)) = (
            self.path.len().checked_sub(1),
            self.list
                .get_mut(recorded..)
                .filter(|list| !list.is_empty()),
        ) else {
            return;
        };
        let subscript = subscript();
        for violation in violations {
            if let Some(element) = violation
                .field
                .as_option_mut()
                .and_then(|field| field.elements.get_mut(depth))
            {
                element.subscript.clone_from(&subscript);
            }
        }
    }
}

/// One validation.
pub(crate) struct Walk<'a, R: Runtime> {
    pool: &'a DescriptorPool,
    schema: &'a Schema,
    env: &'a RwLock<Env>,
    validators: &'a ValidatorCache<R>,
    out: Violations<'a>,
    runtime: PhantomData<R>,
}

impl<'a, R: Runtime> Walk<'a, R> {
    pub(crate) fn new(
        descriptors: &'a Descriptors,
        env: &'a RwLock<Env>,
        validators: &'a ValidatorCache<R>,
        fail_fast: bool,
    ) -> Self {
        Self {
            pool: &descriptors.pool,
            schema: &descriptors.schema,
            env,
            validators,
            out: Violations::new(fail_fast),
            runtime: PhantomData,
        }
    }

    /// Validates `message`, of type `type_name`, returning the violations.
    /// The message is serialized, for CEL, only if a rule needs it.
    pub(crate) fn validate(
        mut self,
        validator: &'a MessageValidator<R>,
        message: &R::Message<'_>,
        type_name: &str,
    ) -> Result<Vec<Violation>, Error<R::Error>> {
        let frame = CelFrame::new(self.env, type_name, message);
        match validator.validate(&mut self, message, &frame) {
            Ok(()) | Err(Abort::FailFast) => Ok(self.out.list),
            Err(Abort::Error(error)) => Err(error),
        }
    }

    /// Runs `f` with `element` appended to the field path.
    fn at<T>(
        &mut self,
        element: &'a FieldPathElement,
        f: impl FnOnce(&mut Self) -> Result<T, Abort<R::Error>>,
    ) -> Result<T, Abort<R::Error>> {
        self.out.path.push(PathElement {
            element,
            subscript: element.subscript.clone(),
        });
        let result = f(self);
        self.out.path.pop();
        result
    }

    /// Runs `f` validating a map key.
    fn for_key<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Abort<R::Error>>,
    ) -> Result<T, Abort<R::Error>> {
        self.out.for_key = true;
        let result = f(self);
        self.out.for_key = false;
        result
    }

    /// The validator of a message type reachable from the root, or why
    /// its rules did not compile.
    fn validator(&self, index: MessageIndex) -> Result<&'a MessageValidator<R>, Abort<R::Error>> {
        let validators = self.validators;
        match validators.get(&index) {
            Some(Ok(validator)) => Ok(validator),
            Some(Err(error)) => Err(Error::Compilation(error.clone()).into()),
            None => Err(Error::Unexpected(format!(
                "rules not loaded for message: {}",
                self.pool.message(index).full_name()
            ))
            .into()),
        }
    }
}

/// A message as CEL sees it, parsed on first use.
///
/// Frames nest on the stack the way the walk does, so one outlives every
/// program run against it.
pub(crate) struct CelFrame<'a, 'm, R: Runtime> {
    env: &'a RwLock<Env>,
    type_name: &'a str,
    message: &'a R::Message<'m>,
    parsed: OnceCell<Frame>,
}

impl<'a, 'm, R: Runtime> CelFrame<'a, 'm, R> {
    fn new(env: &'a RwLock<Env>, type_name: &'a str, message: &'a R::Message<'m>) -> Self {
        Self {
            env,
            type_name,
            message,
            parsed: OnceCell::new(),
        }
    }

    /// The parsed message.
    fn parsed(&self) -> Result<&Frame, Abort<R::Error>> {
        if let Some(frame) = self.parsed.get() {
            return Ok(frame);
        }
        let bytes = self.message.encode().read()?;
        let frame = self
            .env
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .frame(self.type_name, &bytes)?;
        Ok(self.parsed.get_or_init(|| frame))
    }
}

/// Where a value's CEL rules get `this` from.
enum CelBind<'v, 'a, 'm, R: Runtime> {
    /// A field of the frame's message.
    Field(&'v CelFrame<'a, 'm, R>, &'v Field<R>),
    /// One element, key or map value, of this kind.
    Item(Singular),
}

// Only references: copied freely, whatever the runtime is.
impl<R: Runtime> Clone for CelBind<'_, '_, '_, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: Runtime> Copy for CelBind<'_, '_, '_, R> {}

impl<R: Runtime> MessageValidator<R> {
    fn validate<'a>(
        &'a self,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        frame: &CelFrame<'_, '_, R>,
    ) -> Result<(), Abort<R::Error>> {
        if let Some(cel) = &self.cel {
            cel.run(This::Message(frame.parsed()?), &mut walk.out)?;
        }
        for oneof in &self.message_oneofs {
            oneof.check(walk, message)?;
        }
        for oneof in &self.oneofs {
            oneof.check(walk, message)?;
        }
        for field in &self.fields {
            let field = field
                .as_ref()
                .map_err(|error| Error::Compilation(error.clone()))?;
            field.validate(walk, message, frame)?;
        }
        Ok(())
    }
}

impl CelPrograms {
    /// Runs each expression against `this`, recording its failures as
    /// violations. An expression passes by producing `true` or an empty
    /// string. `false` fails with the rule's own message, and a non-empty
    /// string fails with that string as the message.
    fn run<E>(&self, this: This<'_>, out: &mut Violations<'_>) -> Result<(), Abort<E>> {
        for (index, meta) in self.rules.iter().enumerate() {
            let message = match self.program.eval(index, this)? {
                Value::Bool(true) => continue,
                Value::Bool(false) if meta.message.is_empty() => {
                    format!("\"{}\" returned false", meta.expression)
                }
                Value::Bool(false) => meta.message.clone(),
                Value::String(text) if text.is_empty() => continue,
                Value::String(text) => text,
                Value::Other => {
                    return Err(Error::Evaluation("invalid result type".to_owned()).into());
                }
            };
            out.push(&meta.id, &message, &meta.rule_path)?;
        }
        Ok(())
    }
}

impl<R: Runtime> MessageOneof<R> {
    fn check(
        &self,
        walk: &mut Walk<'_, R>,
        message: &R::Message<'_>,
    ) -> Result<(), Abort<R::Error>> {
        let mut set = 0;
        for field in &self.fields {
            if has::<R>(message, field)? {
                set += 1;
            }
        }
        if set > 1 {
            let message = format!("only one of {} can be set", self.names);
            walk.out.push("message.oneof", &message, &[])?;
        }
        if self.required && set == 0 {
            let message = format!("one of {} must be set", self.names);
            walk.out.push("message.oneof", &message, &[])?;
        }
        Ok(())
    }
}

impl<R: Runtime> OneofRequired<R> {
    fn check<'a>(
        &'a self,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
    ) -> Result<(), Abort<R::Error>> {
        for member in &self.members {
            if has::<R>(message, member)? {
                return Ok(());
            }
        }
        walk.at(&self.element, |walk| {
            walk.out
                .push("required", "exactly one field is required in oneof", &[])
        })
    }
}

impl<R: Runtime> FieldValidator<R> {
    /// Validates one field.
    fn validate<'a>(
        &'a self,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        frame: &CelFrame<'_, '_, R>,
    ) -> Result<(), Abort<R::Error>> {
        let value = message.get(&self.field).read()?;
        let set = if self.field.has_presence() {
            message.has(&self.field).read()?
        } else {
            is_set::<R>(value.as_ref())?
        };
        if !set {
            if self.required {
                let required = std::slice::from_ref(&walk.schema.required);
                return walk.at(&self.element, |walk| {
                    walk.out.push("required", "value is required", required)
                });
            }
            if self.ignore_empty {
                return Ok(());
            }
        }

        walk.at(&self.element, |walk| {
            self.value
                .validate(walk, value.as_ref(), CelBind::Field(frame, &self.field))
        })?;

        match (&self.kind, &value, self.field.kind()) {
            (
                FieldKind::List { items: Some(items) },
                Some(Val::List(list)),
                Kind::List(element),
            ) => walk.at(&self.element, |walk| {
                items.validate_list(walk, list, element)
            }),
            (
                FieldKind::Map {
                    element,
                    keys,
                    values,
                },
                Some(Val::Map(map)),
                Kind::Map { key, value: entry },
            ) => walk.at(element, |walk| {
                let entries = Entries {
                    key: Singular::Scalar(key),
                    value: entry,
                    keys: keys.as_deref(),
                    values: values.as_deref(),
                };
                validate_map(walk, map, &entries)
            }),
            _ => Ok(()),
        }
    }
}

impl<R: Runtime> ItemValidator<R> {
    /// Validates each element of a list, of kind `element`, subscripted by
    /// its index.
    fn validate_list(
        &self,
        walk: &mut Walk<'_, R>,
        list: &R::List<'_>,
        element: Singular,
    ) -> Result<(), Abort<R::Error>> {
        let len = list.len().read()?;
        for index in 0..len {
            let Some(item) = list.get(index).read()? else {
                continue;
            };
            if self.ignore_empty && is_empty_item::<R>(&item)? {
                continue;
            }
            walk.out.subscript(Some(Subscript::Index(index as u64)));
            self.value
                .validate(walk, Some(&item), CelBind::Item(element))?;
        }
        Ok(())
    }
}

/// A map field's entries, and their validators.
struct Entries<'a, R: Runtime> {
    key: Singular,
    value: Singular,
    keys: Option<&'a ItemValidator<R>>,
    values: Option<&'a ItemValidator<R>>,
}

/// Validates each entry of a map, subscripted by its key.
fn validate_map<R: Runtime>(
    walk: &mut Walk<'_, R>,
    map: &R::Map<'_>,
    entries: &Entries<'_, R>,
) -> Result<(), Abort<R::Error>> {
    let mut result = Ok(());
    let visited = map.for_each(
        |key, value| match validate_entry(walk, entries, &key, &value) {
            Ok(()) => ControlFlow::Continue(()),
            Err(abort) => {
                result = Err(abort);
                ControlFlow::Break(())
            }
        },
    );
    visited.read()?;
    result
}

fn validate_entry<R: Runtime>(
    walk: &mut Walk<'_, R>,
    entries: &Entries<'_, R>,
    key: &Val<'_, R>,
    value: &Val<'_, R>,
) -> Result<(), Abort<R::Error>> {
    let recorded = walk.out.list.len();
    let result = validate_entry_values(walk, entries, key, value);
    walk.out.subscript_since(recorded, || key_subscript(key));
    result
}

fn validate_entry_values<R: Runtime>(
    walk: &mut Walk<'_, R>,
    entries: &Entries<'_, R>,
    key: &Val<'_, R>,
    value: &Val<'_, R>,
) -> Result<(), Abort<R::Error>> {
    if let Some(keys) = entries.keys {
        if !(keys.ignore_empty && is_empty_item::<R>(key)?) {
            walk.for_key(|walk| {
                keys.value
                    .validate(walk, Some(key), CelBind::Item(entries.key))
            })?;
        }
    }
    if let Some(values) = entries.values {
        if !(values.ignore_empty && is_empty_item::<R>(value)?) {
            values
                .value
                .validate(walk, Some(value), CelBind::Item(entries.value))?;
        }
    }
    Ok(())
}

impl<R: Runtime> ValueValidator<R> {
    /// Validates one value, then the message it holds.
    fn validate(
        &self,
        walk: &mut Walk<'_, R>,
        value: Option<&Val<'_, R>>,
        bind: CelBind<'_, '_, '_, R>,
    ) -> Result<(), Abort<R::Error>> {
        let sub = match value {
            Some(Val::Message(sub)) => Some(sub),
            _ => None,
        };
        // A message value is bound to `this`, and validated, through a
        // frame of its own, parsed at most once.
        let nested = match (sub, self.nested) {
            (Some(sub), Some(index)) => {
                let type_name = walk.pool.message(index).full_name();
                Some((
                    walk.validator(index)?,
                    CelFrame::new(walk.env, type_name, sub),
                ))
            }
            _ => None,
        };
        let child = nested.as_ref().map(|(_, child)| child);

        if let Some(custom) = &self.custom {
            custom.run(cel_this(value, bind, child)?, &mut walk.out)?;
        }
        self.checks.check(walk, value, self.wrapper.as_ref())?;
        if let Some(predefined) = &self.predefined {
            predefined.run(cel_this(value, bind, child)?, &mut walk.out)?;
        }
        if let (Some(sub), Some((validator, child))) = (sub, &nested) {
            validator.validate(walk, sub, child)?;
        }
        Ok(())
    }
}

/// What `this` is bound to: a scalar value directly, and a message, list
/// or map through the parsed frame it lives in.
fn cel_this<'v, R: Runtime>(
    value: Option<&'v Val<'_, R>>,
    bind: CelBind<'v, '_, '_, R>,
    child: Option<&'v CelFrame<'_, '_, R>>,
) -> Result<This<'v>, Abort<R::Error>> {
    let kind = match bind {
        CelBind::Field(_, field) => match field.kind() {
            Kind::Singular(kind) => kind,
            Kind::List(_) | Kind::Map { .. } => Singular::Message,
        },
        CelBind::Item(kind) => kind,
    };
    if let Some(scalar) = cel_scalar(value, kind) {
        return Ok(This::Scalar(scalar));
    }
    match (bind, child) {
        (CelBind::Field(frame, field), _) => Ok(This::Field(
            frame.parsed()?,
            descriptors::field_number(field.number()),
        )),
        (CelBind::Item(_), Some(child)) => Ok(This::Message(child.parsed()?)),
        (CelBind::Item(_), None) => {
            Err(Error::Unexpected("no message to bind this to".to_owned()).into())
        }
    }
}

impl Checks {
    /// Runs the standard checks against a value. A wrapper message is
    /// checked through its `value` field, matching how CEL unboxes wrappers.
    fn check<R: Runtime>(
        &self,
        walk: &mut Walk<'_, R>,
        value: Option<&Val<'_, R>>,
        wrapper: Option<&Field<R>>,
    ) -> Result<(), Abort<R::Error>> {
        match (wrapper, value) {
            (Some(wrapper), Some(Val::Message(message))) => {
                let unboxed = message.get(wrapper).read()?;
                self.check_value(walk, unboxed.as_ref())
            }
            (Some(_), _) => self.check_value(walk, None),
            (None, value) => self.check_value(walk, value),
        }
    }

    fn check_value<R: Runtime>(
        &self,
        walk: &mut Walk<'_, R>,
        value: Option<&Val<'_, R>>,
    ) -> Result<(), Abort<R::Error>> {
        let pool = walk.pool;
        let out = &mut walk.out;
        match self {
            Self::None => Ok(()),
            Self::Bool(checks) => run(checks, &bool_of(value)?, out),
            Self::Int(checks) => run(checks, &int_of(value)?, out),
            Self::Uint(checks) => run(checks, &uint_of(value)?, out),
            Self::Double(checks) => run(checks, &double_of(value)?, out),
            Self::String(checks) => run(checks, str_of(value)?, out),
            Self::Bytes(checks) => run(checks, bytes_of(value)?, out),
            Self::Duration(checks) => run(checks, &Duration(nanos_of(value)?), out),
            Self::Timestamp(checks) => run(checks, &Timestamp(nanos_of(value)?), out),
            Self::FieldMask(checks) => run(checks, paths_of(value)?.as_slice(), out),
            Self::Any(checks) => {
                let field = wkt::any_type_url();
                let type_url = match value {
                    Some(Val::Message(any)) => any.get(&field).read()?,
                    None => None,
                    Some(other) => return Err(mismatch("an Any", other)),
                };
                let type_url = match &type_url {
                    Some(Val::String(url)) => url.as_ref(),
                    _ => "",
                };
                run(checks, type_url, out)
            }
            Self::Enum(checks) => {
                let number = enum_of(value)?;
                run_with(checks, out, |test| {
                    Ok(match test {
                        EnumTest::Cmp(cmp) => evaluate(cmp, &i64::from(number))?,
                        EnumTest::DefinedOnly(index) => {
                            pool.enumeration(*index).value(number).is_none()
                        }
                    })
                })
            }
            Self::List(checks) => {
                let list = match value {
                    Some(Val::List(list)) => Some(list),
                    None => None,
                    Some(other) => return Err(mismatch("a list", other)),
                };
                let len = match list {
                    Some(list) => list.len().read()?,
                    None => 0,
                };
                run_with(checks, out, |test| {
                    Ok(match (test, list) {
                        (ListTest::MinItems(n), _) => (len as u64) < *n,
                        (ListTest::MaxItems(n), _) => len as u64 > *n,
                        (ListTest::Unique, Some(list)) => has_duplicate_items::<R>(list, len)?,
                        (ListTest::Unique, None) => false,
                    })
                })
            }
            Self::Map(checks) => {
                let len = match value {
                    Some(Val::Map(map)) => map.len().read()?,
                    None => 0,
                    Some(other) => return Err(mismatch("a map", other)),
                };
                run_with(checks, out, |test| {
                    Ok(match test {
                        MapTest::MinPairs(n) => (len as u64) < *n,
                        MapTest::MaxPairs(n) => len as u64 > *n,
                    })
                })
            }
        }
    }
}

/// Runs checks of one type against the value they test.
fn run<T, V, E>(checks: &[Check<T>], value: &V, out: &mut Violations<'_>) -> Result<(), Abort<E>>
where
    T: Test<V>,
    V: ?Sized,
{
    run_with(checks, out, |test| evaluate(test, value))
}

/// Runs checks whose test needs more than the value, recording a violation
/// for each one `fails` reports.
fn run_with<T, E>(
    checks: &[Check<T>],
    out: &mut Violations<'_>,
    mut fails: impl FnMut(&T) -> Result<bool, Abort<E>>,
) -> Result<(), Abort<E>> {
    for check in checks {
        if fails(&check.test)? {
            out.push(&check.id, &check.message, &check.rule_path)?;
        }
    }
    Ok(())
}

/// Whether a test fails, or why it could not be evaluated.
fn evaluate<T, V, E>(test: &T, value: &V) -> Result<bool, Abort<E>>
where
    T: Test<V>,
    V: ?Sized,
{
    test.fails(value)
        .map_err(|message| Error::Evaluation(message).into())
}

/// Whether a field is set.
fn has<R: Runtime>(message: &R::Message<'_>, field: &Field<R>) -> Result<bool, Abort<R::Error>> {
    if field.has_presence() {
        return message.has(field).read();
    }
    is_set::<R>(message.get(field).read()?.as_ref())
}

/// Whether a field that does not track presence is set. A double is
/// compared by its bits, so a negative zero counts as set, since it
/// serializes. `None` is a field the message does not have at all.
fn is_set<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<bool, Abort<R::Error>> {
    Ok(match value {
        None => false,
        Some(Val::Bool(b)) => *b,
        Some(Val::Int(i)) => *i != 0,
        Some(Val::Uint(u)) => *u != 0,
        Some(Val::Double(f)) => f.to_bits() != 0,
        Some(Val::Enum(e)) => *e != 0,
        Some(Val::String(s)) => !s.is_empty(),
        Some(Val::Bytes(b)) => !b.is_empty(),
        Some(Val::Message(_)) => true,
        Some(Val::List(list)) => list.len().read()? != 0,
        Some(Val::Map(map)) => map.len().read()? != 0,
    })
}

/// Whether a repeated element or map value is its type's zero value, as
/// `IGNORE_IF_ZERO_VALUE` defines it.
fn is_empty_item<R: Runtime>(value: &Val<'_, R>) -> Result<bool, Abort<R::Error>> {
    Ok(match value {
        Val::Bool(b) => !b,
        Val::Int(i) => *i == 0,
        Val::Uint(u) => *u == 0,
        Val::Double(f) => *f == 0.0,
        Val::Enum(e) => *e == 0,
        Val::String(s) => s.is_empty(),
        Val::Bytes(b) => b.is_empty(),
        Val::Message(message) => message.encode().read()?.is_empty(),
        Val::List(_) | Val::Map(_) => false,
    })
}

/// The subscript for a map entry, which is its key.
fn key_subscript<R: Runtime>(key: &Val<'_, R>) -> Option<Subscript> {
    match key {
        Val::Bool(b) => Some(Subscript::BoolKey(*b)),
        Val::Int(i) => Some(Subscript::IntKey(*i)),
        Val::Uint(u) => Some(Subscript::UintKey(*u)),
        Val::String(s) => Some(Subscript::StringKey(s.to_string())),
        Val::Double(_)
        | Val::Enum(_)
        | Val::Bytes(_)
        | Val::Message(_)
        | Val::List(_)
        | Val::Map(_) => None,
    }
}

/// The CEL scalar for a value of `kind`, or the type's zero when the
/// message does not have the field. `None` when the kind is a message.
fn cel_scalar<'v, R: Runtime>(
    value: Option<&'v Val<'_, R>>,
    kind: Singular,
) -> Option<ScalarValue<'v>> {
    Some(match value {
        Some(Val::Bool(b)) => ScalarValue::Bool(*b),
        Some(Val::Int(i)) => ScalarValue::Int(*i),
        Some(Val::Uint(u)) => ScalarValue::Uint(*u),
        Some(Val::Double(f)) => ScalarValue::Double(*f),
        Some(Val::Enum(e)) => ScalarValue::Int(i64::from(*e)),
        Some(Val::String(s)) => ScalarValue::String(s),
        Some(Val::Bytes(b)) => ScalarValue::Bytes(b),
        Some(Val::Message(_) | Val::List(_) | Val::Map(_)) | None => match kind {
            Singular::Scalar(Scalar::Bool) => ScalarValue::Bool(false),
            Singular::Scalar(
                Scalar::Int32
                | Scalar::Int64
                | Scalar::Sint32
                | Scalar::Sint64
                | Scalar::Sfixed32
                | Scalar::Sfixed64,
            )
            | Singular::Enum => ScalarValue::Int(0),
            Singular::Scalar(
                Scalar::Uint32 | Scalar::Uint64 | Scalar::Fixed32 | Scalar::Fixed64,
            ) => ScalarValue::Uint(0),
            Singular::Scalar(Scalar::Float | Scalar::Double) => ScalarValue::Double(0.0),
            Singular::Scalar(Scalar::String) => ScalarValue::String(""),
            Singular::Scalar(Scalar::Bytes) => ScalarValue::Bytes(&[]),
            Singular::Message => return None,
        },
    })
}

// The typed reads of a value the checks test. `None` is a field the
// message does not have at all, which reads as the type's default. Any
// other type is a runtime handing out a value its descriptor does not
// declare.

fn mismatch<R: Runtime>(expected: &str, got: &Val<'_, R>) -> Abort<R::Error> {
    let got = match got {
        Val::Bool(_) => "a bool",
        Val::Int(_) => "an int",
        Val::Uint(_) => "a uint",
        Val::Double(_) => "a double",
        Val::Enum(_) => "an enum",
        Val::String(_) => "a string",
        Val::Bytes(_) => "bytes",
        Val::Message(_) => "a message",
        Val::List(_) => "a list",
        Val::Map(_) => "a map",
    };
    Error::Unexpected(format!("expected {expected}, the runtime read {got}")).into()
}

fn bool_of<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<bool, Abort<R::Error>> {
    match value {
        None => Ok(false),
        Some(Val::Bool(b)) => Ok(*b),
        Some(other) => Err(mismatch("a bool", other)),
    }
}

fn int_of<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<i64, Abort<R::Error>> {
    match value {
        None => Ok(0),
        Some(Val::Int(i)) => Ok(*i),
        Some(Val::Enum(e)) => Ok(i64::from(*e)),
        Some(other) => Err(mismatch("an int", other)),
    }
}

fn uint_of<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<u64, Abort<R::Error>> {
    match value {
        None => Ok(0),
        Some(Val::Uint(u)) => Ok(*u),
        Some(other) => Err(mismatch("a uint", other)),
    }
}

fn double_of<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<f64, Abort<R::Error>> {
    match value {
        None => Ok(0.0),
        Some(Val::Double(f)) => Ok(*f),
        Some(other) => Err(mismatch("a double", other)),
    }
}

fn enum_of<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<i32, Abort<R::Error>> {
    match value {
        None => Ok(0),
        Some(Val::Enum(e)) => Ok(*e),
        Some(other) => Err(mismatch("an enum", other)),
    }
}

fn str_of<'v, R: Runtime>(value: Option<&'v Val<'_, R>>) -> Result<&'v str, Abort<R::Error>> {
    match value {
        None => Ok(""),
        Some(Val::String(s)) => Ok(s),
        Some(other) => Err(mismatch("a string", other)),
    }
}

fn bytes_of<'v, R: Runtime>(value: Option<&'v Val<'_, R>>) -> Result<&'v [u8], Abort<R::Error>> {
    match value {
        None => Ok(&[]),
        Some(Val::Bytes(b)) => Ok(b),
        Some(other) => Err(mismatch("bytes", other)),
    }
}

/// The `seconds` and `nanos` of a `Duration` or `Timestamp` message
/// combined into nanoseconds. Zero for a missing message.
fn nanos_of<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<i128, Abort<R::Error>> {
    let message = match value {
        None => return Ok(0),
        Some(Val::Message(message)) => message,
        Some(other) => return Err(mismatch("a message", other)),
    };
    let seconds = match message.get(&wkt::seconds()).read()? {
        Some(Val::Int(seconds)) => seconds,
        _ => 0,
    };
    let nanos = match message.get(&wkt::nanos()).read()? {
        Some(Val::Int(nanos)) => nanos,
        _ => 0,
    };
    Ok(total_nanos(seconds, nanos))
}

/// The `paths` of a `FieldMask` message.
fn paths_of<R: Runtime>(value: Option<&Val<'_, R>>) -> Result<Vec<String>, Abort<R::Error>> {
    let message = match value {
        None => return Ok(Vec::new()),
        Some(Val::Message(message)) => message,
        Some(other) => return Err(mismatch("a message", other)),
    };
    let field = wkt::field_mask_paths();
    let Some(Val::List(list)) = message.get(&field).read()? else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    for index in 0..list.len().read()? {
        if let Some(Val::String(path)) = list.get(index).read()? {
            paths.push(path.into_owned());
        }
    }
    Ok(paths)
}

/// Whether a list has two elements that `unique()` considers equal.
fn has_duplicate_items<R: Runtime>(
    list: &R::List<'_>,
    len: usize,
) -> Result<bool, Abort<R::Error>> {
    let mut items = Vec::with_capacity(len);
    for index in 0..len {
        if let Some(item) = list.get(index).read()? {
            items.push(item);
        }
    }
    Ok(has_duplicates(items.iter().map(unique_key)))
}

fn unique_key<'a, R: Runtime>(value: &'a Val<'_, R>) -> Option<UniqueKey<'a>> {
    match value {
        Val::Bool(b) => Some(UniqueKey::Bool(*b)),
        Val::Int(i) => Some(UniqueKey::Int(*i)),
        Val::Enum(e) => Some(UniqueKey::Int(i64::from(*e))),
        Val::Uint(u) => Some(UniqueKey::Uint(*u)),
        Val::Double(f) => UniqueKey::double(*f),
        Val::String(s) => Some(UniqueKey::Str(s)),
        Val::Bytes(b) => Some(UniqueKey::Bytes(b)),
        Val::Message(_) | Val::List(_) | Val::Map(_) => None,
    }
}
