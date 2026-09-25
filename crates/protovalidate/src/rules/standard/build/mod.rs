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

//! Building the checks of one rules message.
//!
//! [`checks`] takes a rules message and returns the checks that
//! `validate.proto` attaches to the rule fields it has native checks for,
//! typed by the rules message. There is usually one check per field, two
//! for the well-known string formats with their `_empty` companion, and
//! one for a pair of bounds, which together spell out a range. The fields
//! it reads are removed from the message, so whatever remains has no
//! native check and is left to its predefined CEL. The messages are
//! formatted here from the rule values, as the CEL expressions would
//! format them. This module has the comparison rules; [`string`] and
//! [`misc`] have the rest.

mod misc;
mod string;

use std::fmt::Display;
use std::mem;

use buffa_descriptor::EnumIndex;
use buffa_types::google::protobuf::{Duration as DurationPb, Timestamp as TimestampPb};
use regex::Regex;

use super::format::{Duration, List, Timestamp, total_nanos};
use super::{Check, Checks, Cmp, DoubleTest, MaybeNan, NowTest, TimestampTest};
use crate::validate::__buffa::oneof;
use crate::validate::__buffa::oneof::field_rules::Type as RulesType;
use crate::validate::FieldPathElement;
use crate::validate::TimestampRules;

/// A check without its rule path, which is added once the rule field it
/// comes from is known.
pub(crate) struct Native<T> {
    pub id: String,
    pub message: String,
    pub test: T,
}

impl<T> Native<T> {
    fn new(prefix: &str, suffix: &str, message: impl Display, test: T) -> Self {
        Self {
            id: format!("{prefix}.{suffix}"),
            message: message.to_string(),
            test,
        }
    }

    fn map<U>(self, f: impl FnOnce(T) -> U) -> Native<U> {
        Native {
            id: self.id,
            message: self.message,
            test: f(self.test),
        }
    }
}

/// The checks of one rules message as they are collected, each with the
/// number of the rule field it comes from, before their rule paths are
/// known. `prefix` is the type in the rule ids, such as `string`.
pub(crate) struct Unplaced<'a, T> {
    prefix: &'a str,
    checks: Vec<(u32, Native<T>)>,
}

impl<'a, T> Unplaced<'a, T> {
    fn new(prefix: &'a str) -> Self {
        Self {
            prefix,
            checks: Vec::new(),
        }
    }

    /// Adds the check of rule field `number`, with id `prefix.suffix`.
    fn push(&mut self, number: u32, suffix: &str, message: impl Display, test: T) {
        self.checks
            .push((number, Native::new(self.prefix, suffix, message, test)));
    }

    fn extend(&mut self, other: Self) {
        self.checks.extend(other.checks);
    }

    fn map<U>(self, f: impl Fn(T) -> U) -> Unplaced<'a, U> {
        Unplaced {
            prefix: self.prefix,
            checks: self
                .checks
                .into_iter()
                .map(|(number, native)| (number, native.map(&f)))
                .collect(),
        }
    }
}

/// How the checks of a rules message are placed.
pub(crate) struct Placement<'a> {
    /// The rule path of a rule field, by its number in the rules message.
    pub path: &'a dyn Fn(u32) -> Vec<FieldPathElement>,
    /// For enum rules, the enum the values belong to.
    pub r#enum: Option<EnumIndex>,
}

impl Placement<'_> {
    fn place<T>(&self, unplaced: Unplaced<'_, T>) -> Vec<Check<T>> {
        unplaced
            .checks
            .into_iter()
            .map(|(number, native)| Check {
                id: native.id,
                message: native.message,
                rule_path: (self.path)(number),
                test: native.test,
            })
            .collect()
    }
}

/// The comparison rules of a numeric, duration or timestamp rules message,
/// with the values already in the type they are compared in.
struct Bounds<T> {
    r#const: Option<T>,
    lt: Option<T>,
    lte: Option<T>,
    gt: Option<T>,
    gte: Option<T>,
    r#in: Vec<T>,
    not_in: Vec<T>,
}

/// One bound of a range, with the words the rule's message and id use for
/// it.
struct Side<T> {
    value: T,
    name: &'static str,
    words: &'static str,
    /// The rule field's number.
    number: u32,
    inclusive: bool,
}

/// Takes the [`Bounds`] out of a rules message whose `less_than` and
/// `greater_than` oneofs live in `$module`; `$const`, `$in` and `$not_in`
/// take the fields outside the oneofs, whose shape varies by message.
macro_rules! bounds {
    ($rules:expr, $module:ident, $convert:expr, $const:expr, $in:expr, $not_in:expr) => {{
        let convert = $convert;
        let (lt, lte) = match $rules.less_than.take() {
            Some(oneof::$module::LessThan::Lt(v)) => (Some(convert(&v)), None),
            Some(oneof::$module::LessThan::Lte(v)) => (None, Some(convert(&v))),
            _ => (None, None),
        };
        let (gt, gte) = match $rules.greater_than.take() {
            Some(oneof::$module::GreaterThan::Gt(v)) => (Some(convert(&v)), None),
            Some(oneof::$module::GreaterThan::Gte(v)) => (None, Some(convert(&v))),
            _ => (None, None),
        };
        Bounds {
            r#const: $const.map(|v| convert(&v)),
            lt,
            lte,
            gt,
            gte,
            r#in: $in.iter().map(convert).collect(),
            not_in: $not_in.iter().map(convert).collect(),
        }
    }};
}

/// The [`bounds!`] of a numeric rules message, whose `const`, `in` and
/// `not_in` are plain fields.
macro_rules! numeric_bounds {
    ($rules:expr, $module:ident, $convert:expr) => {
        bounds!(
            $rules,
            $module,
            $convert,
            $rules.r#const.take(),
            mem::take(&mut $rules.r#in),
            mem::take(&mut $rules.not_in)
        )
    };
}

impl<T: Copy + PartialOrd + MaybeNan + Display> Bounds<T> {
    /// The comparison checks. `first` is the field number of `const`; `lt`,
    /// `lte`, `gt`, `gte`, `in` and `not_in` follow it in that order in
    /// every rules message.
    fn checks<'a>(&self, prefix: &'a str, first: u32) -> Unplaced<'a, Cmp<T>> {
        let mut checks = Unplaced::new(prefix);
        if let Some(c) = self.r#const {
            checks.push(first, "const", format!("must equal {c}"), Cmp::Const(c));
        }
        checks.checks.extend(self.bound(prefix, first));
        if !self.r#in.is_empty() {
            checks.push(
                first + 5,
                "in",
                format!("must be in list {}", List(&self.r#in)),
                Cmp::In(self.r#in.clone()),
            );
        }
        if !self.not_in.is_empty() {
            checks.push(
                first + 6,
                "not_in",
                format!("must not be in list {}", List(&self.not_in)),
                Cmp::NotIn(self.not_in.clone()),
            );
        }
        checks
    }

    /// The bound check. A lower or upper bound alone is checked on its own.
    /// Two together are checked as a range, reported on the lower one: the
    /// values between them when the bounds are in order, the values outside
    /// them when the upper is below the lower. None for bounds that do not
    /// order, which is NaN.
    fn bound(&self, prefix: &str, first: u32) -> Option<(u32, Native<Cmp<T>>)> {
        let side = |value, name, words, number, inclusive| Side {
            value,
            name,
            words,
            number: first + number,
            inclusive,
        };
        let upper = match (self.lt, self.lte) {
            (Some(lt), _) => Some(side(lt, "lt", "less than", 1, false)),
            (None, Some(lte)) => Some(side(lte, "lte", "less than or equal to", 2, true)),
            (None, None) => None,
        };
        let lower = match (self.gt, self.gte) {
            (Some(gt), _) => Some(side(gt, "gt", "greater than", 3, false)),
            (None, Some(gte)) => Some(side(gte, "gte", "greater than or equal to", 4, true)),
            (None, None) => None,
        };
        let (lower, upper) = match (lower, upper) {
            (None, None) => return None,
            (Some(lower), None) => {
                let cmp = if lower.inclusive {
                    Cmp::Gte(lower.value)
                } else {
                    Cmp::Gt(lower.value)
                };
                let message = format!("must be {} {}", lower.words, lower.value);
                return Some((lower.number, Native::new(prefix, lower.name, message, cmp)));
            }
            (None, Some(upper)) => {
                let cmp = if upper.inclusive {
                    Cmp::Lte(upper.value)
                } else {
                    Cmp::Lt(upper.value)
                };
                let message = format!("must be {} {}", upper.words, upper.value);
                return Some((upper.number, Native::new(prefix, upper.name, message, cmp)));
            }
            (Some(lower), Some(upper)) => (lower, upper),
        };
        let (lo, hi) = (lower.value, upper.value);
        let (suffix, message, cmp) = if hi >= lo {
            (
                format!("{}_{}", lower.name, upper.name),
                format!("must be {} {lo} and {} {hi}", lower.words, upper.words),
                match (lower.inclusive, upper.inclusive) {
                    (false, false) => Cmp::GtLt { gt: lo, lt: hi },
                    (false, true) => Cmp::GtLte { gt: lo, lte: hi },
                    (true, false) => Cmp::GteLt { gte: lo, lt: hi },
                    (true, true) => Cmp::GteLte { gte: lo, lte: hi },
                },
            )
        } else if hi < lo {
            (
                format!("{}_{}_exclusive", lower.name, upper.name),
                format!("must be {} {lo} or {} {hi}", lower.words, upper.words),
                match (lower.inclusive, upper.inclusive) {
                    (false, false) => Cmp::GtLtExclusive { gt: lo, lt: hi },
                    (false, true) => Cmp::GtLteExclusive { gt: lo, lte: hi },
                    (true, false) => Cmp::GteLtExclusive { gte: lo, lt: hi },
                    (true, true) => Cmp::GteLteExclusive { gte: lo, lte: hi },
                },
            )
        } else {
            return None;
        };
        Some((lower.number, Native::new(prefix, &suffix, message, cmp)))
    }
}

fn duration(d: &DurationPb) -> Duration {
    Duration(total_nanos(d.seconds, i64::from(d.nanos)))
}

fn timestamp(t: &TimestampPb) -> Timestamp {
    Timestamp(total_nanos(t.seconds, i64::from(t.nanos)))
}

pub(super) fn regex(pattern: &str) -> Result<Regex, String> {
    Regex::new(pattern).map_err(|error| format!("invalid regex pattern `{pattern}`: {error}"))
}

/// The placed checks of an integer rules message holding `$narrow` values,
/// compared as `$wide`: the type CEL widens the integer to, which every
/// comparison and range shares.
macro_rules! integer_checks {
    ($variant:ident, $at:expr, $prefix:expr, $rules:expr, $module:ident, $narrow:ty => $wide:ty) => {
        Checks::$variant($at.place(
            numeric_bounds!($rules, $module, |v: &$narrow| <$wide>::from(*v)).checks($prefix, 1),
        ))
    };
}

/// The native checks of the rule fields set in `rules`, placed by `at`, in
/// the order `validate.proto` declares the fields. The fields read are
/// removed from `rules`. `prefix` is the type in the rule ids, such as
/// `string`. Fails for a rule that cannot be compiled, such as an invalid
/// regex.
pub(crate) fn checks(
    prefix: &str,
    rules: &mut RulesType,
    at: &Placement<'_>,
) -> Result<Checks, String> {
    Ok(match rules {
        RulesType::Float(r) => {
            let bounds = numeric_bounds!(r, float_rules, |v: &f32| f64::from(*v));
            Checks::Double(at.place(floating(prefix, &bounds, r.finite.take())))
        }
        RulesType::Double(r) => {
            let bounds = numeric_bounds!(r, double_rules, |v: &f64| *v);
            Checks::Double(at.place(floating(prefix, &bounds, r.finite.take())))
        }
        RulesType::Int32(r) => integer_checks!(Int, at, prefix, r, int32rules, i32 => i64),
        RulesType::Int64(r) => integer_checks!(Int, at, prefix, r, int64rules, i64 => i64),
        RulesType::Uint32(r) => integer_checks!(Uint, at, prefix, r, u_int32rules, u32 => u64),
        RulesType::Uint64(r) => integer_checks!(Uint, at, prefix, r, u_int64rules, u64 => u64),
        RulesType::Sint32(r) => integer_checks!(Int, at, prefix, r, s_int32rules, i32 => i64),
        RulesType::Sint64(r) => integer_checks!(Int, at, prefix, r, s_int64rules, i64 => i64),
        RulesType::Fixed32(r) => integer_checks!(Uint, at, prefix, r, fixed32rules, u32 => u64),
        RulesType::Fixed64(r) => integer_checks!(Uint, at, prefix, r, fixed64rules, u64 => u64),
        RulesType::Sfixed32(r) => integer_checks!(Int, at, prefix, r, s_fixed32rules, i32 => i64),
        RulesType::Sfixed64(r) => integer_checks!(Int, at, prefix, r, s_fixed64rules, i64 => i64),
        RulesType::Duration(r) => Checks::Duration(
            at.place(
                bounds!(
                    r,
                    duration_rules,
                    duration,
                    r.r#const.take(),
                    mem::take(&mut r.r#in),
                    mem::take(&mut r.not_in)
                )
                .checks(prefix, 2),
            ),
        ),
        RulesType::Timestamp(r) => Checks::Timestamp(at.place(timestamp_checks(prefix, r))),
        RulesType::Bool(r) => Checks::Bool(at.place(misc::bool_checks(prefix, r))),
        RulesType::Enum(r) => Checks::Enum(at.place(misc::enum_checks(prefix, r, at.r#enum))),
        RulesType::String(r) => Checks::String(at.place(string::checks(prefix, r)?)),
        RulesType::Bytes(r) => Checks::Bytes(at.place(misc::bytes_checks(prefix, r)?)),
        RulesType::Repeated(r) => Checks::List(at.place(misc::repeated_checks(prefix, r))),
        RulesType::Map(r) => Checks::Map(at.place(misc::map_checks(prefix, r))),
        RulesType::FieldMask(r) => Checks::FieldMask(at.place(misc::field_mask_checks(prefix, r))),
        RulesType::Any(r) => Checks::Any(at.place(misc::any_checks(prefix, r))),
    })
}

/// The checks of a float or double rules message, which add `finite` to
/// the comparisons.
fn floating<'a>(
    prefix: &'a str,
    bounds: &Bounds<f64>,
    finite: Option<bool>,
) -> Unplaced<'a, DoubleTest> {
    let mut checks = bounds.checks(prefix, 1).map(DoubleTest::Cmp);
    if finite == Some(true) {
        checks.push(8, "finite", "must be finite", DoubleTest::Finite);
    }
    checks
}

/// The checks of a timestamp rules message. `lt_now` and `gt_now` take the
/// place of a bound in their oneofs.
fn timestamp_checks<'a>(prefix: &'a str, r: &mut TimestampRules) -> Unplaced<'a, TimestampTest> {
    let mut checks = Unplaced::new(prefix);
    if let Some(oneof::timestamp_rules::LessThan::LtNow(lt_now)) = r.less_than {
        r.less_than = None;
        if lt_now {
            checks.push(
                7,
                "lt_now",
                "must be less than now",
                TimestampTest::Now(NowTest::LtNow),
            );
        }
    }
    let gt_now = match r.greater_than {
        Some(oneof::timestamp_rules::GreaterThan::GtNow(gt_now)) => {
            r.greater_than = None;
            gt_now
        }
        _ => false,
    };
    let none: [TimestampPb; 0] = [];
    let bounds = bounds!(r, timestamp_rules, timestamp, r.r#const.take(), none, none);
    checks.extend(bounds.checks(prefix, 2).map(TimestampTest::Cmp));
    if gt_now {
        checks.push(
            8,
            "gt_now",
            "must be greater than now",
            TimestampTest::Now(NowTest::GtNow),
        );
    }
    if let Some(within) = r.within.take() {
        let within = duration(&within);
        checks.push(
            9,
            "within",
            format!("must be within {within} of now"),
            TimestampTest::Now(NowTest::Within(within)),
        );
    }
    checks
}
