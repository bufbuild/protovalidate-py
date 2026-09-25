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

//! The bool, enum, bytes, repeated, map, field mask and any rules.

use std::mem;

use buffa_descriptor::EnumIndex;

use super::{Unplaced, regex};
use crate::rules::standard::format::{Hex, List, Text};
use crate::rules::standard::{AnyTest, BytesTest, Cmp, EnumTest, FieldMaskTest, ListTest, MapTest};
use crate::validate::__buffa::oneof;
use crate::validate::{
    AnyRules, BoolRules, BytesRules, EnumRules, FieldMaskRules, MapRules, RepeatedRules,
};

pub(super) fn bool_checks<'a>(prefix: &'a str, r: &mut BoolRules) -> Unplaced<'a, bool> {
    let mut checks = Unplaced::new(prefix);
    if let Some(c) = r.r#const.take() {
        checks.push(1, "const", format!("must equal {c}"), c);
    }
    checks
}

/// The enum checks. `defined_only` is checked against the values of
/// `enum_`, the field's enum type, which the type check has already
/// confirmed.
pub(super) fn enum_checks<'a>(
    prefix: &'a str,
    r: &mut EnumRules,
    enum_: Option<EnumIndex>,
) -> Unplaced<'a, EnumTest> {
    let ints = |list: Vec<i32>| list.into_iter().map(i64::from).collect::<Vec<_>>();
    let mut checks = Unplaced::new(prefix);
    if let Some(c) = r.r#const.take() {
        checks.push(
            1,
            "const",
            format!("must equal {c}"),
            EnumTest::Cmp(Cmp::Const(i64::from(c))),
        );
    }
    if let (Some(true), Some(enum_)) = (r.defined_only.take(), enum_) {
        checks.push(
            2,
            "defined_only",
            "value must be one of the defined enum values",
            EnumTest::DefinedOnly(enum_),
        );
    }
    if !r.r#in.is_empty() {
        let list = ints(mem::take(&mut r.r#in));
        checks.push(
            3,
            "in",
            format!("must be in list {}", List(&list)),
            EnumTest::Cmp(Cmp::In(list)),
        );
    }
    if !r.not_in.is_empty() {
        let list = ints(mem::take(&mut r.not_in));
        checks.push(
            4,
            "not_in",
            format!("must not be in list {}", List(&list)),
            EnumTest::Cmp(Cmp::NotIn(list)),
        );
    }
    checks
}

pub(super) fn bytes_checks<'a>(
    prefix: &'a str,
    r: &mut BytesRules,
) -> Result<Unplaced<'a, BytesTest>, String> {
    let mut checks = Unplaced::new(prefix);
    if let Some(c) = r.r#const.take() {
        checks.push(
            1,
            "const",
            format!("must be {}", Hex(&c)),
            BytesTest::Const(c),
        );
    }
    if let Some(n) = r.len.take() {
        checks.push(13, "len", format!("must be {n} bytes"), BytesTest::Len(n));
    }
    if let Some(n) = r.min_len.take() {
        checks.push(
            2,
            "min_len",
            format!("must be at least {n} bytes"),
            BytesTest::MinLen(n),
        );
    }
    if let Some(n) = r.max_len.take() {
        checks.push(
            3,
            "max_len",
            format!("must be at most {n} bytes"),
            BytesTest::MaxLen(n),
        );
    }
    if let Some(p) = r.pattern.take() {
        let compiled = regex(&p)?;
        checks.push(
            4,
            "pattern",
            format!("must match regex pattern `{p}`"),
            BytesTest::Pattern(compiled),
        );
    }
    if let Some(p) = r.prefix.take() {
        checks.push(
            5,
            "prefix",
            format!("does not have prefix {}", Hex(&p)),
            BytesTest::Prefix(p),
        );
    }
    if let Some(p) = r.suffix.take() {
        checks.push(
            6,
            "suffix",
            format!("does not have suffix {}", Hex(&p)),
            BytesTest::Suffix(p),
        );
    }
    if let Some(p) = r.contains.take() {
        checks.push(
            7,
            "contains",
            format!("does not contain {}", Hex(&p)),
            BytesTest::Contains(p),
        );
    }
    if !r.r#in.is_empty() {
        let list = mem::take(&mut r.r#in);
        checks.push(
            8,
            "in",
            format!(
                "must be in list {}",
                List(list.iter().map(|bytes| Text(bytes)))
            ),
            BytesTest::In(list),
        );
    }
    if !r.not_in.is_empty() {
        let list = mem::take(&mut r.not_in);
        checks.push(
            9,
            "not_in",
            format!(
                "must not be in list {}",
                List(list.iter().map(|bytes| Text(bytes)))
            ),
            BytesTest::NotIn(list),
        );
    }
    if let Some(format) = r.well_known.take() {
        bytes_format_checks(&mut checks, &format);
    }
    Ok(checks)
}

/// The checks of a bytes format rule, which only look at the length:
/// the format check and its `_empty` companion. A format set to false
/// adds nothing.
fn bytes_format_checks(
    checks: &mut Unplaced<'_, BytesTest>,
    format: &oneof::bytes_rules::WellKnown,
) {
    use oneof::bytes_rules::WellKnown as Wk;
    let (number, suffix, what, test) = match format {
        Wk::Ip(true) => (10, "ip", "IP address", BytesTest::Ip),
        Wk::Ipv4(true) => (11, "ipv4", "IPv4 address", BytesTest::Ipv4),
        Wk::Ipv6(true) => (12, "ipv6", "IPv6 address", BytesTest::Ipv6),
        Wk::Uuid(true) => (15, "uuid", "UUID", BytesTest::Uuid),
        Wk::Ip(false) | Wk::Ipv4(false) | Wk::Ipv6(false) | Wk::Uuid(false) => return,
    };
    checks.push(number, suffix, format!("must be a valid {what}"), test);
    checks.push(
        number,
        &format!("{suffix}_empty"),
        format!("value is empty, which is not a valid {what}"),
        BytesTest::Empty,
    );
}

pub(super) fn repeated_checks<'a>(
    prefix: &'a str,
    r: &mut RepeatedRules,
) -> Unplaced<'a, ListTest> {
    let mut checks = Unplaced::new(prefix);
    if let Some(n) = r.min_items.take() {
        checks.push(
            1,
            "min_items",
            format!("must contain at least {n} item(s)"),
            ListTest::MinItems(n),
        );
    }
    if let Some(n) = r.max_items.take() {
        checks.push(
            2,
            "max_items",
            format!("must contain no more than {n} item(s)"),
            ListTest::MaxItems(n),
        );
    }
    if r.unique.take() == Some(true) {
        checks.push(
            3,
            "unique",
            "repeated value must contain unique items",
            ListTest::Unique,
        );
    }
    // `items` holds the elements' rules, built on their own.
    checks
}

pub(super) fn map_checks<'a>(prefix: &'a str, r: &mut MapRules) -> Unplaced<'a, MapTest> {
    let mut checks = Unplaced::new(prefix);
    if let Some(n) = r.min_pairs.take() {
        checks.push(
            1,
            "min_pairs",
            format!("map must be at least {n} entries"),
            MapTest::MinPairs(n),
        );
    }
    if let Some(n) = r.max_pairs.take() {
        checks.push(
            2,
            "max_pairs",
            format!("map must be at most {n} entries"),
            MapTest::MaxPairs(n),
        );
    }
    // `keys` and `values` hold the entries' rules, built on their own.
    checks
}

pub(super) fn field_mask_checks<'a>(
    prefix: &'a str,
    r: &mut FieldMaskRules,
) -> Unplaced<'a, FieldMaskTest> {
    let mut checks = Unplaced::new(prefix);
    if let Some(c) = r.r#const.take() {
        checks.push(
            1,
            "const",
            format!("must equal paths {}", List(&c.paths)),
            FieldMaskTest::Const(c.paths),
        );
    }
    if !r.r#in.is_empty() {
        let list = mem::take(&mut r.r#in);
        checks.push(
            2,
            "in",
            format!("must only contain paths in {}", List(&list)),
            FieldMaskTest::In(list),
        );
    }
    if !r.not_in.is_empty() {
        let list = mem::take(&mut r.not_in);
        checks.push(
            3,
            "not_in",
            format!("must not contain any paths in {}", List(&list)),
            FieldMaskTest::NotIn(list),
        );
    }
    checks
}

/// The any checks, on the `type_url`. `in` applies only when it lists
/// something.
pub(super) fn any_checks<'a>(prefix: &'a str, r: &mut AnyRules) -> Unplaced<'a, AnyTest> {
    let mut checks = Unplaced::new(prefix);
    if !r.r#in.is_empty() {
        checks.push(
            2,
            "in",
            "type URL must be in the allow list",
            AnyTest::In(mem::take(&mut r.r#in)),
        );
    }
    if !r.not_in.is_empty() {
        checks.push(
            3,
            "not_in",
            "type URL must not be in the block list",
            AnyTest::NotIn(mem::take(&mut r.not_in)),
        );
    }
    checks
}
