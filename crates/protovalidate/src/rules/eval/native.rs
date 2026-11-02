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

//! Evaluation of the native checks against the values they test.

use std::borrow::Cow;

use buffa_descriptor::DescriptorPool;

use super::{Abort, Read as _, Violations, Walk};
use crate::Error;
use crate::protobuf::{Field, List as _, Map as _, Runtime, Val, read};
use crate::rules::NativeChecks;
use crate::rules::standard::{
    Check, Duration, EnumTest, ListTest, MapTest, ScalarChecks, Test, Timestamp, UniqueKey,
    has_duplicates, total_nanos,
};

impl<R: Runtime> NativeChecks<R> {
    /// Runs the checks against a value, recording a violation for each one
    /// that fails. `None` is a field the message does not have at all,
    /// which is checked as its type's default.
    pub(super) fn check(
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
pub(super) fn run<T, V, E>(
    checks: &[Check<T>],
    value: &V,
    out: &mut Violations,
) -> Result<(), Abort<E>>
where
    T: Test<V>,
    V: ?Sized,
{
    run_with(checks, out, |test| evaluate(test, value))
}

/// Runs checks whose test needs more than the value, recording a violation
/// for each one `fails` reports.
pub(super) fn run_with<T, E>(
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
pub(super) fn evaluate<T, V, E>(test: &T, value: &V) -> Result<bool, Abort<E>>
where
    T: Test<V>,
    V: ?Sized,
{
    test.fails(value)
        .map_err(|message| Error::Evaluation(message).into())
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
/// combined into nanoseconds. Zero for a missing message.
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
