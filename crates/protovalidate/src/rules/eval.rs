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
//! path the walk is at and the rule path compiled into the check. A read
//! that fails ends the walk early, as does the first violation when failing
//! fast. Either one propagates as an [`Abort`] through `?`.

use std::borrow::Cow;
use std::cell::OnceCell;
use std::ops::ControlFlow;

use buffa_descriptor::{DescriptorPool, MessageIndex};

use super::standard::{
    Check, Duration, EnumTest, ListTest, MapTest, ScalarChecks, Test, Timestamp, UniqueKey,
    has_duplicates, total_nanos,
};
use super::{
    CelPrograms, FieldHeader, FieldKind, FieldValidator, ItemValidator, MessageOneof,
    MessageValidator, NativeChecks, OneofValidator, Step, ValidatorCache, ValidatorIndex,
    ValueValidator,
};
use crate::Error;
use crate::cel::{self, Env, Frame, ScalarValue, This, Value};
use crate::descriptors::{self, Descriptors, Schema};
use crate::protobuf::{
    Field, Key, Kind, List as _, Map as _, Message as _, Runtime, Scalar, Singular, Val, read,
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

/// The violations found so far. A violation's field path is built as the
/// walk returns from each field, so validating a valid message builds no
/// paths.
pub(crate) struct Violations {
    fail_fast: bool,
    /// Whether the value being validated is a map key.
    for_key: bool,
    list: Vec<Violation>,
}

impl Violations {
    fn new(fail_fast: bool) -> Self {
        Self {
            fail_fast,
            for_key: false,
            list: Vec::new(),
        }
    }

    /// Records a violation of the value being validated. Its field path is
    /// filled in by the fields that the walk returns through. When failing
    /// fast, the walk stops here.
    fn push<E>(
        &mut self,
        id: &str,
        message: String,
        rule_path: &[FieldPathElement],
    ) -> Result<(), Abort<E>> {
        let rule = (!rule_path.is_empty()).then(|| FieldPath {
            elements: rule_path.to_vec(),
            ..Default::default()
        });
        self.list.push(Violation {
            rule: rule.into(),
            rule_id: Some(id.to_owned()),
            message: Some(message),
            for_key: self.for_key.then_some(true),
            ..Default::default()
        });
        if self.fail_fast {
            Err(Abort::FailFast)
        } else {
            Ok(())
        }
    }

    /// Appends `element`, with the subscript that `subscript` computes, to
    /// the field path of each violation recorded since index `recorded`.
    /// The subscript is only computed if there is such a violation. Paths
    /// are built from the innermost field outward, and [`Self::finish`]
    /// puts them in order.
    fn append<E>(
        &mut self,
        recorded: usize,
        element: &FieldPathElement,
        subscript: impl FnOnce() -> Result<Option<Subscript>, Abort<E>>,
    ) -> Result<(), Abort<E>> {
        let Some(violations) = self
            .list
            .get_mut(recorded..)
            .filter(|list| !list.is_empty())
        else {
            return Ok(());
        };
        let mut element = element.clone();
        element.subscript = subscript()?;
        for violation in violations {
            violation
                .field
                .get_or_insert_default()
                .elements
                .push(element.clone());
        }
        Ok(())
    }

    /// The violations, with their field paths starting from the root.
    fn finish(mut self) -> Vec<Violation> {
        for violation in &mut self.list {
            if let Some(field) = violation.field.as_option_mut() {
                field.elements.reverse();
            }
        }
        self.list
    }
}

/// One validation.
pub(crate) struct Walk<'a, R: Runtime> {
    pool: &'a DescriptorPool,
    schema: &'a Schema,
    env: &'a Env,
    validators: &'a ValidatorCache<R>,
    out: Violations,
}

impl<'a, R: Runtime> Walk<'a, R> {
    pub(crate) fn new(
        descriptors: &'a Descriptors,
        env: &'a Env,
        validators: &'a ValidatorCache<R>,
        fail_fast: bool,
    ) -> Self {
        Self {
            pool: &descriptors.pool,
            schema: &descriptors.schema,
            env,
            validators,
            out: Violations::new(fail_fast),
        }
    }

    /// Validates `message`, of type `message_type`, and returns the
    /// violations. The message is only serialized for CEL if a rule needs
    /// it.
    pub(crate) fn validate(
        mut self,
        validator: &'a MessageValidator<R>,
        message: &R::Message<'_>,
        message_type: MessageIndex,
    ) -> Result<Vec<Violation>, Error<R::Error>> {
        let frame = CelFrame::new(self.env, self.pool, message_type, message);
        match validator.validate(&mut self, message, &frame) {
            Ok(()) | Err(Abort::FailFast) => Ok(self.out.finish()),
            Err(Abort::Error(error)) => Err(error),
        }
    }

    /// Runs `f` within the field of `element`, adding that field to the
    /// paths of the violations that `f` records.
    fn at<T>(
        &mut self,
        element: &'a FieldPathElement,
        f: impl FnOnce(&mut Self) -> Result<T, Abort<R::Error>>,
    ) -> Result<T, Abort<R::Error>> {
        self.at_item(element, || Ok(element.subscript.clone()), f)
    }

    /// Runs `f` within one item of a container field, as [`at`](Self::at)
    /// does within a field. The item's subscript is only computed, by
    /// `subscript`, if `f` records a violation.
    fn at_item<T>(
        &mut self,
        element: &'a FieldPathElement,
        subscript: impl FnOnce() -> Result<Option<Subscript>, Abort<R::Error>>,
        f: impl FnOnce(&mut Self) -> Result<T, Abort<R::Error>>,
    ) -> Result<T, Abort<R::Error>> {
        let recorded = self.out.list.len();
        // Whether `f` failed or the walk is stopping, the violations it
        // recorded are within this field.
        let result = f(self);
        let appended = self.out.append(recorded, element, subscript);
        match (result, appended) {
            // An error from `f` happened first, so it takes precedence.
            (Err(Abort::Error(error)), _) => Err(Abort::Error(error)),
            // Reading the subscript failed, which is reported even if the
            // walk was already stopping.
            (_, Err(error)) => Err(error),
            (result, Ok(())) => result,
        }
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

    /// The validator of a message type reachable from the root, along with
    /// the message type, or why its rules did not compile.
    fn validator(
        &self,
        index: ValidatorIndex,
    ) -> Result<(MessageIndex, &'a MessageValidator<R>), Abort<R::Error>> {
        match self.validators.validator(index) {
            (message, Ok(validator)) => Ok((message, validator)),
            (_, Err(error)) => Err(Error::Compilation(error.clone()).into()),
        }
    }
}

/// A message as CEL sees it, parsed on first use.
///
/// Frames nest on the stack the way the walk does, so one outlives every
/// program run against it.
pub(crate) struct CelFrame<'a, 'm, R: Runtime> {
    env: &'a Env,
    pool: &'a DescriptorPool,
    message_type: MessageIndex,
    message: &'a R::Message<'m>,
    parsed: OnceCell<Frame>,
}

impl<'a, 'm, R: Runtime> CelFrame<'a, 'm, R> {
    fn new(
        env: &'a Env,
        pool: &'a DescriptorPool,
        message_type: MessageIndex,
        message: &'a R::Message<'m>,
    ) -> Self {
        Self {
            env,
            pool,
            message_type,
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
        let type_name = self.pool.message(self.message_type).full_name();
        let frame = self.env.frame(type_name, bytes.as_ref())?;
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
        for step in &self.steps {
            match step
                .as_ref()
                .map_err(|error| Error::Compilation(error.clone()))?
            {
                Step::Field(field) => field.validate(walk, message, frame, None)?,
                Step::Oneof(oneof) => oneof.validate(walk, message, frame)?,
            }
        }
        Ok(())
    }
}

impl CelPrograms {
    /// Runs each expression against `this`, recording its failures as
    /// violations. An expression passes by producing `true` or an empty
    /// string. `false` fails with the rule's own message, and a non-empty
    /// string fails with that string as the message.
    fn run<E>(&self, this: This<'_>, out: &mut Violations) -> Result<(), Abort<E>> {
        for (index, meta) in self.rules.iter().enumerate() {
            let value = self.program.eval(index, this).map_err(|error| {
                Error::Evaluation(format!("error evaluating {}: {}", meta.id, error.message()))
            })?;
            let message = match value {
                Value::Bool(true) => continue,
                Value::Bool(false) if meta.message.is_empty() => {
                    format!("\"{}\" returned false", meta.expression)
                }
                Value::Bool(false) => meta.message.clone(),
                Value::String(text) if text.is_empty() => continue,
                Value::String(text) => text,
                Value::Other => {
                    return Err(Error::Evaluation(format!(
                        "expression {} outputs an unexpected type, wanted either bool or string",
                        meta.id
                    ))
                    .into());
                }
            };
            out.push(&meta.id, message, &meta.rule_path)?;
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
            walk.out.push("message.oneof", message, &[])?;
        }
        if self.required && set == 0 {
            let message = format!("one of {} must be set", self.names);
            walk.out.push("message.oneof", message, &[])?;
        }
        Ok(())
    }
}

impl<R: Runtime> OneofValidator<R> {
    /// Validates the set member, if there is one. If none is set and the
    /// oneof is required, reports a violation.
    fn validate<'a>(
        &'a self,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        frame: &CelFrame<'_, '_, R>,
    ) -> Result<(), Abort<R::Error>> {
        if let Some(number) = message.which_oneof(&self.oneof).read()? {
            // A member that the validator does not know of has no rules.
            return match self.members.iter().find(|(member, _)| *member == number) {
                Some((_, field)) => field.validate(walk, message, frame, Some(true)),
                None => Ok(()),
            };
        }
        let Some(element) = &self.required else {
            return Ok(());
        };
        walk.at(element, |walk| {
            walk.out.push(
                "required",
                "exactly one field is required in oneof".to_owned(),
                &[],
            )
        })
    }
}

impl<R: Runtime> FieldValidator<R> {
    /// Validates one field. `present` is whether the field is set, if the
    /// caller already knows, as it does for the set member of a oneof.
    fn validate<'a>(
        &'a self,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        frame: &CelFrame<'_, '_, R>,
        present: Option<bool>,
    ) -> Result<(), Abort<R::Error>> {
        match self {
            Self::Presence(header) => header.validate_presence(walk, message, present),
            Self::Scalar(header, checks) => header.validate_scalar(checks, walk, message, present),
            Self::Message(header, nested) => {
                header.validate_message(*nested, walk, message, present)
            }
            Self::Value(header, value, kind) => {
                header.validate_value(value, kind, walk, message, frame, present)
            }
        }
    }
}

impl<R: Runtime> FieldHeader<R> {
    /// Returns whether the field is set, when `present` already says so or
    /// the field tracks presence. For any other field, it returns `None`,
    /// because whether the field is set depends on its value.
    fn presence(
        &self,
        message: &R::Message<'_>,
        present: Option<bool>,
    ) -> Result<Option<bool>, Abort<R::Error>> {
        Ok(match present {
            Some(present) => Some(present),
            None if self.field.has_presence() => Some(message.has(&self.field).read()?),
            None => None,
        })
    }

    /// Validates a [`FieldValidator::Presence`] field, whose value no rule
    /// reads. It only needs to be set, and only if it is required.
    fn validate_presence<'a>(
        &'a self,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        present: Option<bool>,
    ) -> Result<(), Abort<R::Error>> {
        if !self.required {
            return Ok(());
        }
        let set = match self.presence(message, present)? {
            Some(set) => set,
            None => is_set::<R>(read::<R>(message, &self.field).read()?.as_ref())?,
        };
        if set {
            return Ok(());
        }
        self.missing(walk)
    }

    /// Validates a [`FieldValidator::Scalar`] field. The kind of `checks` is
    /// the field's kind, so the value is read with the
    /// [`Message`](crate::protobuf::Message) method for that kind rather
    /// than as a [`Val`].
    fn validate_scalar<'a>(
        &'a self,
        checks: &'a ScalarChecks,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        present: Option<bool>,
    ) -> Result<(), Abort<R::Error>> {
        let present = self.presence(message, present)?;
        let field = &self.field;
        match checks {
            ScalarChecks::Bool(checks) => self.scalar(
                walk,
                present,
                || message.get_bool(field),
                |value| *value,
                |out, value| run(checks, value, out),
            ),
            ScalarChecks::Int(checks) => self.scalar(
                walk,
                present,
                || message.get_int(field),
                |value| *value != 0,
                |out, value| run(checks, value, out),
            ),
            ScalarChecks::Uint(checks) => self.scalar(
                walk,
                present,
                || message.get_uint(field),
                |value| *value != 0,
                |out, value| run(checks, value, out),
            ),
            ScalarChecks::Double(checks) => self.scalar(
                walk,
                present,
                || message.get_double(field),
                |value| value.to_bits() != 0,
                |out, value| run(checks, value, out),
            ),
            ScalarChecks::String(checks) => self.scalar(
                walk,
                present,
                || message.get_str(field),
                |value| !value.is_empty(),
                |out, value| run(checks, &**value, out),
            ),
            ScalarChecks::Bytes(checks) => self.scalar(
                walk,
                present,
                || message.get_bytes(field),
                |value| !value.is_empty(),
                |out, value| run(checks, &**value, out),
            ),
            ScalarChecks::Enum(checks) => {
                let pool = walk.pool;
                self.scalar(
                    walk,
                    present,
                    || message.get_enum(field),
                    |value| *value != 0,
                    |out, &number| {
                        run_with(checks, out, |test| {
                            Ok(match test {
                                EnumTest::Cmp(cmp) => evaluate(cmp, &i64::from(number))?,
                                EnumTest::DefinedOnly(index) => {
                                    pool.enumeration(*index).value(number).is_none()
                                }
                            })
                        })
                    },
                )
            }
        }
    }

    /// Applies the presence handling and checks of
    /// [`Self::validate_scalar`] to a value of type `T`. It only calls `get`
    /// to read the value if the value is needed.
    fn scalar<'a, T: Default>(
        &'a self,
        walk: &mut Walk<'a, R>,
        present: Option<bool>,
        get: impl FnOnce() -> Result<Option<T>, R::Error>,
        nonzero: impl FnOnce(&T) -> bool,
        check: impl FnOnce(&mut Violations, &T) -> Result<(), Abort<R::Error>>,
    ) -> Result<(), Abort<R::Error>> {
        let got = if present == Some(false) && (self.required || self.ignore_empty) {
            None
        } else {
            get().read()?
        };
        let set = match present {
            Some(present) => present,
            None => got.as_ref().is_some_and(nonzero),
        };
        if !set {
            if self.required {
                return self.missing(walk);
            }
            if self.ignore_empty {
                return Ok(());
            }
        }
        // If the runtime's message does not have the field, its default
        // value is checked, as when the field is read as a `Val`.
        let value = got.unwrap_or_default();
        walk.at(&self.element, |walk| check(&mut walk.out, &value))
    }

    /// Validates a [`FieldValidator::Message`] field. An unset field is
    /// skipped unless it is required, and a set field is validated by the
    /// rules of its message type `nested` alone, without a [`Val`].
    fn validate_message<'a>(
        &'a self,
        nested: ValidatorIndex,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        present: Option<bool>,
    ) -> Result<(), Abort<R::Error>> {
        let set = match present {
            Some(present) => present,
            None => message.has(&self.field).read()?,
        };
        if !set {
            return if self.required {
                self.missing(walk)
            } else {
                Ok(())
            };
        }
        // A field that the runtime's message does not have holds no message
        // to validate.
        let Some(sub) = message.get_message(&self.field).read()? else {
            return Ok(());
        };
        let (message_type, validator) = walk.validator(nested)?;
        let frame = CelFrame::new(walk.env, walk.pool, message_type, &sub);
        walk.at(&self.element, |walk| validator.validate(walk, &sub, &frame))
    }

    /// Records that the field is required but not set.
    fn missing<'a>(&'a self, walk: &mut Walk<'a, R>) -> Result<(), Abort<R::Error>> {
        let required = std::slice::from_ref(&walk.schema.required);
        walk.at(&self.element, |walk| {
            walk.out
                .push("required", "value is required".to_owned(), required)
        })
    }

    /// Validates a [`FieldValidator::Value`] field through its value, read
    /// as a [`Val`], and then through the validators of its items.
    fn validate_value<'a>(
        &'a self,
        value: &'a ValueValidator<R>,
        kind: &'a FieldKind<R>,
        walk: &mut Walk<'a, R>,
        message: &R::Message<'_>,
        frame: &CelFrame<'_, '_, R>,
        present: Option<bool>,
    ) -> Result<(), Abort<R::Error>> {
        let present = self.presence(message, present)?;
        // An unset field whose value would not be used is not read. In
        // particular, this skips unset oneof members and message fields.
        let val = if present == Some(false) && (self.required || self.ignore_empty) {
            None
        } else {
            read::<R>(message, &self.field).read()?
        };
        // Whether the field is set only matters if it is required or ignored
        // when empty, and for a container, finding out means reading its
        // size.
        if self.required || self.ignore_empty {
            let set = match present {
                Some(present) => present,
                None => is_set::<R>(val.as_ref())?,
            };
            if !set {
                return if self.required {
                    self.missing(walk)
                } else {
                    Ok(())
                };
            }
        }

        walk.at(&self.element, |walk| {
            value.validate(walk, val.as_ref(), CelBind::Field(frame, &self.field))
        })?;

        match (kind, &val, self.field.kind()) {
            (
                FieldKind::List { items: Some(items) },
                Some(Val::List(list)),
                Kind::List(element),
            ) => items.validate_list(walk, &self.element, list, element),
            (
                FieldKind::Map {
                    element,
                    keys,
                    values,
                },
                Some(Val::Map(map)),
                Kind::Map { key, value: entry },
            ) => {
                let entries = Entries {
                    element,
                    key: Singular::Scalar(key),
                    value: entry,
                    keys: keys.as_deref(),
                    values: values.as_deref(),
                };
                validate_map(walk, map, &entries)
            }
            _ => Ok(()),
        }
    }
}

impl<R: Runtime> ItemValidator<R> {
    /// Validates each element of a list, of kind `element`, as an item of
    /// the field `field`, with its index as the subscript.
    fn validate_list<'a>(
        &'a self,
        walk: &mut Walk<'a, R>,
        field: &'a FieldPathElement,
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
            walk.at_item(
                field,
                || Ok(Some(Subscript::Index(index as u64))),
                |walk| {
                    self.value
                        .validate(walk, Some(&item), CelBind::Item(element))
                },
            )?;
        }
        Ok(())
    }
}

/// A map field's entries, and their validators.
struct Entries<'a, R: Runtime> {
    /// The field's path element, with the key and value types filled in.
    element: &'a FieldPathElement,
    key: Singular,
    value: Singular,
    keys: Option<&'a ItemValidator<R>>,
    values: Option<&'a ItemValidator<R>>,
}

/// Validates each entry of a map, subscripted by its key.
fn validate_map<'a, R: Runtime>(
    walk: &mut Walk<'a, R>,
    map: &R::Map<'_>,
    entries: &Entries<'a, R>,
) -> Result<(), Abort<R::Error>> {
    let visited = map
        .for_each(
            |key, value| match validate_entry(walk, entries, key, &value) {
                Ok(()) => ControlFlow::Continue(()),
                Err(abort) => ControlFlow::Break(abort),
            },
        )
        .read()?;
    visited.break_value().map_or(Ok(()), Err)
}

fn validate_entry<'a, 'k, R: Runtime>(
    walk: &mut Walk<'a, R>,
    entries: &Entries<'a, R>,
    key: Key<'k, R>,
    value: &Val<'k, R>,
) -> Result<(), Abort<R::Error>> {
    // The key is read at most once, when key rules or a violation's path
    // first need it.
    let read = OnceCell::new();
    walk.at_item(
        entries.element,
        || Ok(key_subscript(entry_key(&read, key)?)),
        |walk| validate_entry_values(walk, entries, &read, key, value),
    )
}

/// Returns the entry's key from `read`, reading it with `key` first if it
/// has not been read yet.
fn entry_key<'r, 'k, R: Runtime>(
    read: &'r OnceCell<Val<'k, R>>,
    key: Key<'k, R>,
) -> Result<&'r Val<'k, R>, Abort<R::Error>> {
    if let Some(value) = read.get() {
        return Ok(value);
    }
    let value = key().read()?;
    Ok(read.get_or_init(|| value))
}

fn validate_entry_values<'k, R: Runtime>(
    walk: &mut Walk<'_, R>,
    entries: &Entries<'_, R>,
    read: &OnceCell<Val<'k, R>>,
    key: Key<'k, R>,
    value: &Val<'k, R>,
) -> Result<(), Abort<R::Error>> {
    if let Some(keys) = entries.keys {
        let key = entry_key(read, key)?;
        if !(keys.ignore_empty && is_empty_item::<R>(key)?) {
            walk.for_key(|walk| {
                keys.value
                    .validate(walk, Some(key), CelBind::Item(entries.key))
            })?;
        }
    }
    if let Some(values) = entries.values
        && !(values.ignore_empty && is_empty_item::<R>(value)?)
    {
        values
            .value
            .validate(walk, Some(value), CelBind::Item(entries.value))?;
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
            (Some(sub), Some(nested)) => {
                let (message_type, validator) = walk.validator(nested)?;
                Some((
                    validator,
                    CelFrame::new(walk.env, walk.pool, message_type, sub),
                ))
            }
            _ => None,
        };
        let child = nested.as_ref().map(|(_, child)| child);

        if self.reads_value() {
            if let Some(custom) = &self.custom {
                custom.run(cel_this(value, bind, child)?, &mut walk.out)?;
            }
            self.native.check(walk, value)?;
            if let Some(predefined) = &self.predefined {
                predefined.run(cel_this(value, bind, child)?, &mut walk.out)?;
            }
        }
        if let (Some(sub), Some((validator, child))) = (sub, &nested) {
            validator.validate(walk, sub, child)?;
        }
        Ok(())
    }
}

/// What `this` is bound to. A scalar value is bound directly, and a
/// message, list or map is bound through the parsed frame it lives in.
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

impl<R: Runtime> NativeChecks<R> {
    /// Runs the checks against a value, recording a violation for each one
    /// that fails. `None` is a field the message does not have at all,
    /// which is checked as its type's default.
    fn check(
        &self,
        walk: &mut Walk<'_, R>,
        value: Option<&Val<'_, R>>,
    ) -> Result<(), Abort<R::Error>> {
        let pool = walk.pool;
        let out = &mut walk.out;
        match self {
            Self::None => Ok(()),
            Self::Scalar(checks) => checks.check(pool, out, value),
            // A wrapper message is checked through its `value` field,
            // matching how CEL unboxes wrappers.
            Self::Wrapped { checks, field } => {
                let unboxed = match value {
                    Some(Val::Message(message)) => read::<R>(message, field).read()?,
                    None => None,
                    Some(other) => return Err(mismatch("a wrapper message", other)),
                };
                checks.check(pool, out, unboxed.as_ref())
            }
            Self::List { checks, element } => {
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
                        (ListTest::Unique, Some(list)) => {
                            has_duplicate_items::<R>(list, len, element.as_ref())?
                        }
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
            Self::Duration {
                checks,
                seconds,
                nanos,
            } => run(checks, &Duration(nanos_of(value, seconds, nanos)?), out),
            Self::Timestamp {
                checks,
                seconds,
                nanos,
            } => run(checks, &Timestamp(nanos_of(value, seconds, nanos)?), out),
            Self::FieldMask { checks, paths } => {
                with_paths(value, paths, |paths| run(checks, paths, out))?
            }
            Self::Any { checks, type_url } => {
                let url = match value {
                    Some(Val::Message(any)) => read::<R>(any, type_url).read()?,
                    None => None,
                    Some(other) => return Err(mismatch("an Any", other)),
                };
                let url = match &url {
                    Some(Val::String(url)) => url.as_ref(),
                    _ => "",
                };
                run(checks, url, out)
            }
        }
    }
}

impl ScalarChecks {
    /// Runs the checks against a scalar or enum value, recording a
    /// violation for each one that fails. `None` is checked as the type's
    /// default.
    fn check<R: Runtime>(
        &self,
        pool: &DescriptorPool,
        out: &mut Violations,
        value: Option<&Val<'_, R>>,
    ) -> Result<(), Abort<R::Error>> {
        match self {
            Self::Bool(checks) => run(checks, &bool_of(value)?, out),
            Self::Int(checks) => run(checks, &int_of(value)?, out),
            Self::Uint(checks) => run(checks, &uint_of(value)?, out),
            Self::Double(checks) => run(checks, &double_of(value)?, out),
            Self::String(checks) => run(checks, str_of(value)?, out),
            Self::Bytes(checks) => run(checks, bytes_of(value)?, out),
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
        }
    }
}

/// Runs checks of one type against the value they test.
fn run<T, V, E>(checks: &[Check<T>], value: &V, out: &mut Violations) -> Result<(), Abort<E>>
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
    out: &mut Violations,
    mut fails: impl FnMut(&T) -> Result<bool, Abort<E>>,
) -> Result<(), Abort<E>> {
    for check in checks {
        if fails(&check.test)? {
            out.push(&check.id, check.message.clone(), &check.rule_path)?;
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
    is_set::<R>(read::<R>(message, field).read()?.as_ref())
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
        Val::Message(message) => message.encode().read()?.as_ref().is_empty(),
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
/// message does not have the field. Returns `None` when the kind is a
/// message.
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

// These functions read the value a check tests as the type the check
// expects. `None` is a field the message does not have at all, which is
// checked as the type's default. A value of any other type means the
// runtime handed out a value its descriptor does not declare.

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
/// combined into nanoseconds. A missing message is zero.
fn nanos_of<R: Runtime>(
    value: Option<&Val<'_, R>>,
    seconds: &Field<R>,
    nanos: &Field<R>,
) -> Result<i128, Abort<R::Error>> {
    let message = match value {
        None => return Ok(0),
        Some(Val::Message(message)) => message,
        Some(other) => return Err(mismatch("a message", other)),
    };
    let seconds = match read::<R>(message, seconds).read()? {
        Some(Val::Int(seconds)) => seconds,
        _ => 0,
    };
    let nanos = match read::<R>(message, nanos).read()? {
        Some(Val::Int(nanos)) => nanos,
        _ => 0,
    };
    Ok(total_nanos(seconds, nanos))
}

/// Runs `f` on the paths of a `FieldMask` message, read from its field
/// `paths`, or on none for a missing message.
fn with_paths<R: Runtime, T>(
    value: Option<&Val<'_, R>>,
    paths: &Field<R>,
    f: impl FnOnce(&[Cow<'_, str>]) -> T,
) -> Result<T, Abort<R::Error>> {
    let message = match value {
        None => return Ok(f(&[])),
        Some(Val::Message(message)) => message,
        Some(other) => return Err(mismatch("a message", other)),
    };
    let Some(Val::List(list)) = read::<R>(message, paths).read()? else {
        return Ok(f(&[]));
    };
    let mut found = Vec::new();
    for index in 0..list.len().read()? {
        if let Some(Val::String(path)) = list.get(index).read()? {
            found.push(path);
        }
    }
    Ok(f(&found))
}

/// Whether a list has two elements that `unique()` considers equal.
/// Wrapper elements are compared by the scalar in their `wrapper` field.
fn has_duplicate_items<R: Runtime>(
    list: &R::List<'_>,
    len: usize,
    wrapper: Option<&Field<R>>,
) -> Result<bool, Abort<R::Error>> {
    let mut items = Vec::with_capacity(len);
    for index in 0..len {
        if let Some(item) = list.get(index).read()? {
            items.push(item);
        }
    }
    let Some(wrapper) = wrapper else {
        return Ok(has_duplicates(items.iter().map(unique_key)));
    };
    let mut unboxed = Vec::with_capacity(items.len());
    for item in &items {
        match item {
            Val::Message(message) => unboxed.push(read::<R>(message, wrapper).read()?),
            other => return Err(mismatch("a wrapper message", other)),
        }
    }
    Ok(has_duplicates(
        unboxed
            .iter()
            .map(|value| value.as_ref().and_then(unique_key)),
    ))
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
