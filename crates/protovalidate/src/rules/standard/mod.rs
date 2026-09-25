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

//! The standard rules, evaluated natively.
//!
//! `validate.proto` defines every standard rule as a CEL expression over
//! `this` and `rules`, with a message formatted from the rule's value. A
//! [`Check`] is one such expression compiled ahead of time. The rules
//! message is known when the field's rules are built, so the message is
//! formatted then, and only the [`Test`] against the value is left for
//! validation. Checks report the same rule ids, messages and rule paths as
//! the CEL expressions, and run in the same order.
//!
//! The tests here are over plain values. Reading those values out of a
//! message is the walk's job, in [`eval`](super::eval).

pub(crate) mod build;
mod format;
pub(crate) mod wellknown;

use std::collections::HashSet;
use std::time::SystemTime;

use buffa_descriptor::EnumIndex;
use regex::Regex;

use crate::validate::FieldPathElement;
pub(crate) use format::{Duration, Timestamp, total_nanos};

/// One standard rule against one value.
pub(crate) struct Check<T> {
    /// The rule id, such as `int32.gt_lt`.
    pub id: String,
    pub message: String,
    /// The rule path, from `FieldRules` down: `[FieldRules.int32,
    /// Int32Rules.gt]`, or `[FieldRules.repeated, RepeatedRules.items,
    /// FieldRules.int32, Int32Rules.gt]` for an element.
    pub rule_path: Vec<FieldPathElement>,
    pub test: T,
}

/// What a check tests a value for.
pub(crate) trait Test<V: ?Sized> {
    /// Whether `value` breaks the rule, or why it could not be checked.
    fn fails(&self, value: &V) -> Result<bool, String>;
}

/// The checks of a `FieldRules`, grouped by the rules message it sets. Each
/// variant holds its checks in evaluation order, typed by the value they
/// test.
#[derive(Default)]
pub(crate) enum Checks {
    /// No rules message is set.
    #[default]
    None,
    Bool(Vec<Check<bool>>),
    Int(Vec<Check<Cmp<i64>>>),
    Uint(Vec<Check<Cmp<u64>>>),
    Double(Vec<Check<DoubleTest>>),
    String(Vec<Check<StrTest>>),
    Bytes(Vec<Check<BytesTest>>),
    Enum(Vec<Check<EnumTest>>),
    List(Vec<Check<ListTest>>),
    Map(Vec<Check<MapTest>>),
    Duration(Vec<Check<Cmp<Duration>>>),
    Timestamp(Vec<Check<TimestampTest>>),
    FieldMask(Vec<Check<FieldMaskTest>>),
    Any(Vec<Check<AnyTest>>),
}

/// A comparison against the rule's values. The range variants are the
/// combinations that `validate.proto` defines for a `gt` or `gte` rule when
/// a `lt` or `lte` is also set.
pub(crate) enum Cmp<T> {
    Const(T),
    Lt(T),
    Lte(T),
    Gt(T),
    Gte(T),
    /// `gt < lt`: the value must be strictly inside.
    GtLt {
        gt: T,
        lt: T,
    },
    /// `lt < gt`: the value must be strictly outside `lt..=gt`.
    GtLtExclusive {
        gt: T,
        lt: T,
    },
    GtLte {
        gt: T,
        lte: T,
    },
    GtLteExclusive {
        gt: T,
        lte: T,
    },
    GteLt {
        gte: T,
        lt: T,
    },
    GteLtExclusive {
        gte: T,
        lt: T,
    },
    GteLte {
        gte: T,
        lte: T,
    },
    GteLteExclusive {
        gte: T,
        lte: T,
    },
    In(Vec<T>),
    NotIn(Vec<T>),
}

/// Values that fail every comparison rule, such as NaN.
pub(crate) trait MaybeNan {
    fn is_nan(&self) -> bool {
        false
    }
}

impl MaybeNan for i64 {}
impl MaybeNan for u64 {}
impl MaybeNan for Duration {}
impl MaybeNan for Timestamp {}
impl MaybeNan for f64 {
    fn is_nan(&self) -> bool {
        f64::is_nan(*self)
    }
}

impl<T: PartialOrd + MaybeNan> Test<T> for Cmp<T> {
    fn fails(&self, value: &T) -> Result<bool, String> {
        if value.is_nan() {
            // NaN is not equal to, in, or ordered against anything: every
            // rule fails except `not_in`, which it trivially satisfies.
            return Ok(!matches!(self, Self::NotIn(_)));
        }
        Ok(match self {
            Self::Const(c) => value != c,
            Self::Lt(lt) => value >= lt,
            Self::Lte(lte) => value > lte,
            Self::Gt(gt) => value <= gt,
            Self::Gte(gte) => value < gte,
            Self::GtLt { gt, lt } => value >= lt || value <= gt,
            Self::GtLtExclusive { gt, lt } => lt <= value && value <= gt,
            Self::GtLte { gt, lte } => value > lte || value <= gt,
            Self::GtLteExclusive { gt, lte } => lte < value && value <= gt,
            Self::GteLt { gte, lt } => value >= lt || value < gte,
            Self::GteLtExclusive { gte, lt } => lt <= value && value < gte,
            Self::GteLte { gte, lte } => value > lte || value < gte,
            Self::GteLteExclusive { gte, lte } => lte < value && value < gte,
            Self::In(list) => !list.contains(value),
            Self::NotIn(list) => list.contains(value),
        })
    }
}

/// The float and double rules.
pub(crate) enum DoubleTest {
    Cmp(Cmp<f64>),
    Finite,
}

impl Test<f64> for DoubleTest {
    fn fails(&self, value: &f64) -> Result<bool, String> {
        match self {
            Self::Cmp(cmp) => cmp.fails(value),
            Self::Finite => Ok(!value.is_finite()),
        }
    }
}

/// `bool.const`.
impl Test<bool> for bool {
    fn fails(&self, value: &bool) -> Result<bool, String> {
        Ok(value != self)
    }
}

pub(crate) enum StrTest {
    Const(String),
    /// In Unicode code points, as CEL's `size()` counts them.
    Len(u64),
    MinLen(u64),
    MaxLen(u64),
    LenBytes(u64),
    MinBytes(u64),
    MaxBytes(u64),
    Pattern(Regex),
    Prefix(String),
    Suffix(String),
    Contains(String),
    NotContains(String),
    In(Vec<String>),
    NotIn(Vec<String>),
    WellKnown(WellKnown),
    /// The `_empty` companion of a well-known format. The format rule
    /// accepts an empty string; this rule rejects it.
    Empty,
}

/// A well-known string format.
pub(crate) enum WellKnown {
    Email,
    Hostname,
    Ip,
    Ipv4,
    Ipv6,
    Uri,
    UriRef,
    Address,
    Uuid,
    Tuuid,
    IpWithPrefixlen,
    Ipv4WithPrefixlen,
    Ipv6WithPrefixlen,
    IpPrefix,
    Ipv4Prefix,
    Ipv6Prefix,
    HostAndPort,
    Ulid,
    ProtobufFqn,
    ProtobufDotFqn,
    HeaderName { strict: bool },
    HeaderValue { strict: bool },
}

impl WellKnown {
    /// Whether the format lets an empty string through, leaving it to the
    /// `_empty` companion check. A URI reference and a header value do not.
    pub(crate) fn allows_empty(&self) -> bool {
        !matches!(self, Self::UriRef | Self::HeaderValue { .. })
    }

    fn fails(&self, s: &str) -> bool {
        if s.is_empty() && self.allows_empty() {
            return false;
        }
        !match self {
            Self::Email => wellknown::is_email(s),
            Self::Hostname => wellknown::is_hostname(s),
            Self::Ip => wellknown::is_ip(s),
            Self::Ipv4 => wellknown::is_ipv4(s),
            Self::Ipv6 => wellknown::is_ipv6(s),
            Self::Uri => wellknown::is_uri(s),
            Self::Address => wellknown::is_hostname(s) || wellknown::is_ip(s),
            Self::Uuid => wellknown::is_uuid(s),
            Self::Tuuid => wellknown::is_tuuid(s),
            Self::IpWithPrefixlen => wellknown::is_ip_prefix(s, false),
            Self::Ipv4WithPrefixlen => wellknown::is_ipv4_prefix(s, false),
            Self::Ipv6WithPrefixlen => wellknown::is_ipv6_prefix(s, false),
            Self::IpPrefix => wellknown::is_ip_prefix(s, true),
            Self::Ipv4Prefix => wellknown::is_ipv4_prefix(s, true),
            Self::Ipv6Prefix => wellknown::is_ipv6_prefix(s, true),
            Self::HostAndPort => wellknown::is_host_and_port(s, true),
            Self::Ulid => wellknown::is_ulid(s),
            Self::ProtobufFqn => wellknown::is_protobuf_fqn(s),
            Self::ProtobufDotFqn => wellknown::is_protobuf_dot_fqn(s),
            Self::HeaderName { strict } => wellknown::is_header_name(s, *strict),
            Self::UriRef => wellknown::is_uri_ref(s),
            Self::HeaderValue { strict } => wellknown::is_header_value(s, *strict),
        }
    }
}

impl Test<str> for StrTest {
    fn fails(&self, s: &str) -> Result<bool, String> {
        Ok(match self {
            Self::Const(c) => s != c,
            Self::Len(n) => chars(s) != *n,
            Self::MinLen(n) => chars(s) < *n,
            Self::MaxLen(n) => chars(s) > *n,
            Self::LenBytes(n) => s.len() as u64 != *n,
            Self::MinBytes(n) => (s.len() as u64) < *n,
            Self::MaxBytes(n) => s.len() as u64 > *n,
            Self::Pattern(regex) => !regex.is_match(s),
            Self::Prefix(p) => !s.starts_with(p.as_str()),
            Self::Suffix(p) => !s.ends_with(p.as_str()),
            Self::Contains(p) => !s.contains(p.as_str()),
            Self::NotContains(p) => s.contains(p.as_str()),
            Self::In(list) => !list.iter().any(|item| item == s),
            Self::NotIn(list) => list.iter().any(|item| item == s),
            Self::WellKnown(format) => format.fails(s),
            Self::Empty => s.is_empty(),
        })
    }
}

fn chars(s: &str) -> u64 {
    s.chars().count() as u64
}

pub(crate) enum BytesTest {
    Const(Vec<u8>),
    Len(u64),
    MinLen(u64),
    MaxLen(u64),
    /// Matched against the bytes decoded as UTF-8, as CEL's `string(this)`
    /// would give.
    Pattern(Regex),
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    Contains(Vec<u8>),
    In(Vec<Vec<u8>>),
    NotIn(Vec<Vec<u8>>),
    /// 4 or 16 bytes, or none.
    Ip,
    Ipv4,
    Ipv6,
    /// 16 bytes, or none.
    Uuid,
    /// The `_empty` companion of the formats above.
    Empty,
}

impl Test<[u8]> for BytesTest {
    /// Bytes that are not UTF-8 cannot be matched against a pattern,
    /// because CEL's `string()` fails on them.
    fn fails(&self, b: &[u8]) -> Result<bool, String> {
        Ok(match self {
            Self::Const(c) => b != c.as_slice(),
            Self::Len(n) => b.len() as u64 != *n,
            Self::MinLen(n) => (b.len() as u64) < *n,
            Self::MaxLen(n) => b.len() as u64 > *n,
            Self::Pattern(regex) => match std::str::from_utf8(b) {
                Ok(s) => !regex.is_match(s),
                Err(_) => return Err("value must be valid UTF-8 to apply regexp".to_owned()),
            },
            Self::Prefix(p) => !b.starts_with(p),
            Self::Suffix(p) => !b.ends_with(p),
            Self::Contains(p) => !contains(b, p),
            Self::In(list) => !list.iter().any(|item| item == b),
            Self::NotIn(list) => list.iter().any(|item| item == b),
            Self::Ip => !matches!(b.len(), 0 | 4 | 16),
            Self::Ipv4 => !matches!(b.len(), 0 | 4),
            Self::Ipv6 => !matches!(b.len(), 0 | 16),
            Self::Uuid => !matches!(b.len(), 0 | 16),
            Self::Empty => b.is_empty(),
        })
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// The enum rules.
pub(crate) enum EnumTest {
    Cmp(Cmp<i64>),
    DefinedOnly(EnumIndex),
}

/// The repeated rules, which the walk evaluates over the list.
pub(crate) enum ListTest {
    MinItems(u64),
    MaxItems(u64),
    Unique,
}

/// The map rules, which the walk evaluates over the map.
pub(crate) enum MapTest {
    MinPairs(u64),
    MaxPairs(u64),
}

/// The `any` rules, on the `type_url`.
pub(crate) enum AnyTest {
    In(Vec<String>),
    NotIn(Vec<String>),
}

impl Test<str> for AnyTest {
    fn fails(&self, type_url: &str) -> Result<bool, String> {
        Ok(match self {
            Self::In(list) => !list.iter().any(|allowed| allowed == type_url),
            Self::NotIn(list) => list.iter().any(|blocked| blocked == type_url),
        })
    }
}

/// The timestamp rules.
pub(crate) enum TimestampTest {
    Cmp(Cmp<Timestamp>),
    Now(NowTest),
}

pub(crate) enum NowTest {
    LtNow,
    GtNow,
    Within(Duration),
}

impl Test<Timestamp> for TimestampTest {
    fn fails(&self, value: &Timestamp) -> Result<bool, String> {
        let now = now();
        Ok(match self {
            Self::Cmp(cmp) => return cmp.fails(value),
            Self::Now(NowTest::LtNow) => *value > now,
            Self::Now(NowTest::GtNow) => *value < now,
            Self::Now(NowTest::Within(within)) => {
                value.0 < now.0 - within.0 || value.0 > now.0 + within.0
            }
        })
    }
}

fn now() -> Timestamp {
    let nanos = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(since) => i128::try_from(since.as_nanos()).unwrap_or(i128::MAX),
        Err(before) => -i128::try_from(before.duration().as_nanos()).unwrap_or(i128::MAX),
    };
    Timestamp(nanos)
}

pub(crate) enum FieldMaskTest {
    Const(Vec<String>),
    /// Every path must be in the list, or below a path in it.
    In(Vec<String>),
    NotIn(Vec<String>),
}

fn covered(paths: &[String], path: &str) -> bool {
    paths.iter().any(|allowed| {
        allowed == path
            || path
                .strip_prefix(allowed.as_str())
                .is_some_and(|rest| rest.starts_with('.'))
    })
}

impl Test<[String]> for FieldMaskTest {
    fn fails(&self, paths: &[String]) -> Result<bool, String> {
        Ok(match self {
            Self::Const(c) => paths != c.as_slice(),
            Self::In(list) => !paths.iter().all(|path| covered(list, path)),
            Self::NotIn(list) => paths.iter().any(|path| covered(list, path)),
        })
    }
}

/// A list element as `unique()` compares it, by type and value. Signed
/// zeros are equal, and NaN is never equal to anything.
#[derive(PartialEq, Eq, Hash)]
pub(crate) enum UniqueKey<'a> {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Double(u64),
    Str(&'a str),
    Bytes(&'a [u8]),
}

impl UniqueKey<'_> {
    /// The key of a double, or `None` for NaN, which never duplicates
    /// anything.
    pub(crate) fn double(value: f64) -> Option<Self> {
        if value.is_nan() {
            return None;
        }
        Some(Self::Double(if value == 0.0 { 0 } else { value.to_bits() }))
    }
}

/// How many keys [`has_duplicates`] compares pairwise before it switches
/// to hashing.
const PAIRWISE_KEYS: usize = 16;

/// Whether any key occurs twice. Elements without a key never count as
/// duplicates.
pub(crate) fn has_duplicates<'a>(keys: impl IntoIterator<Item = Option<UniqueKey<'a>>>) -> bool {
    let mut keys = keys.into_iter().flatten();
    let mut first: [Option<UniqueKey<'a>>; PAIRWISE_KEYS] = [const { None }; PAIRWISE_KEYS];
    for (len, key) in keys.by_ref().enumerate() {
        if first[..len].iter().any(|seen| seen.as_ref() == Some(&key)) {
            return true;
        }
        if len == PAIRWISE_KEYS {
            let mut seen: HashSet<_, foldhash::fast::RandomState> =
                first.iter_mut().filter_map(Option::take).collect();
            seen.insert(key);
            return keys.any(|key| !seen.insert(key));
        }
        first[len] = Some(key);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{PAIRWISE_KEYS, UniqueKey, has_duplicates};

    fn ints(values: impl IntoIterator<Item = i64>) -> Vec<Option<UniqueKey<'static>>> {
        values
            .into_iter()
            .map(|value| Some(UniqueKey::Int(value)))
            .collect()
    }

    #[test]
    fn duplicates_compared_pairwise() {
        assert!(!has_duplicates(ints(0..3)));
        assert!(has_duplicates(ints([1, 2, 1])));
        assert!(!has_duplicates([None, None, Some(UniqueKey::Str("a"))]));
    }

    #[test]
    fn duplicates_hashed() {
        let len = i64::try_from(PAIRWISE_KEYS).unwrap() + 4;
        assert!(!has_duplicates(ints(0..len)));
        assert!(has_duplicates(ints((0..len).chain([len - 1]))));
    }
}
