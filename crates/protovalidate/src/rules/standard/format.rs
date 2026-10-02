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

//! How rule values print in violation messages.
//!
//! The standard rules' messages are `format()` calls in `validate.proto`, so
//! a message quotes the rule's value the way CEL's `%s` and `%x` print it:
//! decimal integers, lists in brackets, durations as seconds with an `s`,
//! timestamps in RFC 3339. Integers, strings and doubles print through
//! [`Display`], timestamps through jiff; the types here cover the rest.
//! Doubles print in the shortest round-trip form, infinities as `Infinity`
//! and `-Infinity`, and fractions of a second with the digits they need.

use std::fmt::{self, Display};

const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// The `seconds` and `nanos` a `Duration` or `Timestamp` message carries,
/// combined into nanoseconds. `nanos` is an `int32`, taken as the runtime
/// widens it.
pub(crate) fn total_nanos(seconds: i64, nanos: i64) -> i128 {
    i128::from(seconds) * NANOS_PER_SECOND + i128::from(nanos)
}

/// A `google.protobuf.Duration`, as nanoseconds.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct Duration(pub i128);

/// A `google.protobuf.Timestamp`, as nanoseconds since the Unix epoch.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct Timestamp(pub i128);

impl Timestamp {
    /// `0001-01-01T00:00:00Z`, the earliest value a `Timestamp` may hold.
    pub(crate) const MIN: Self = Self(-62_135_596_800 * NANOS_PER_SECOND);
    /// `9999-12-31T23:59:59.999999999Z`, the latest.
    pub(crate) const MAX: Self = Self(253_402_300_799 * NANOS_PER_SECOND + 999_999_999);

    /// Whether the value is in the range `google.protobuf.Timestamp` allows.
    pub(crate) fn in_range(self) -> bool {
        (Self::MIN..=Self::MAX).contains(&self)
    }
}

/// A `double` or `float` rule value, compared as an `f64`.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct Double(pub f64);

/// Bytes as `%s` prints them: as text, with what is not UTF-8 replaced.
#[derive(Clone, Copy)]
pub(crate) struct Text<'a>(pub &'a [u8]);

/// Bytes as `%x` prints them: lowercase hex.
#[derive(Clone, Copy)]
pub(crate) struct Hex<'a>(pub &'a [u8]);

/// A list as `%s` prints it: the elements, comma separated, in brackets.
pub(crate) struct List<I>(pub I);

/// A fraction of a second, in the digits it needs; nothing for zero.
fn fraction(f: &mut fmt::Formatter<'_>, mut nanos: u128) -> fmt::Result {
    if nanos == 0 {
        return Ok(());
    }
    let mut width = 9;
    while nanos.is_multiple_of(10) {
        nanos /= 10;
        width -= 1;
    }
    write!(f, ".{nanos:0width$}")
}

/// `1.5s`, `-0.000001s`, `0s`.
impl Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0 {
            f.write_str("-")?;
        }
        let magnitude = self.0.unsigned_abs();
        let nanos_per_second = NANOS_PER_SECOND.unsigned_abs();
        write!(f, "{}", magnitude / nanos_per_second)?;
        fraction(f, magnitude % nanos_per_second)?;
        f.write_str("s")
    }
}

/// RFC 3339 in UTC, with the fractional digits the value needs. A value
/// outside the range a `Timestamp` allows prints as seconds since the
/// epoch.
///
/// The value is added to the epoch as a civil date-time rather than made a
/// `jiff::Timestamp`, whose range stops a day short of `9999-12-31`.
impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let civil = if self.in_range() {
            let seconds = self.0.div_euclid(NANOS_PER_SECOND);
            let nanos = self.0.rem_euclid(NANOS_PER_SECOND);
            i64::try_from(seconds)
                .ok()
                .zip(i64::try_from(nanos).ok())
                .and_then(|(seconds, nanos)| {
                    let span = jiff::Span::new()
                        .try_seconds(seconds)
                        .ok()?
                        .try_nanoseconds(nanos)
                        .ok()?;
                    jiff::civil::date(1970, 1, 1)
                        .at(0, 0, 0, 0)
                        .checked_add(span)
                        .ok()
                })
        } else {
            None
        };
        match civil {
            Some(civil) => write!(f, "{civil}Z"),
            None => write!(f, "{} since the epoch", Duration(self.0)),
        }
    }
}

/// As `%s` prints a double: `Infinity`, `-Infinity`, or the shortest form
/// that reads back to the value.
impl Display for Double {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_infinite() {
            f.write_str(if self.0 > 0.0 {
                "Infinity"
            } else {
                "-Infinity"
            })
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl Display for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.0))
    }
}

impl Display for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl<I> Display for List<I>
where
    I: IntoIterator + Clone,
    I::Item: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for (index, item) in self.0.clone().into_iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{item}")?;
        }
        f.write_str("]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(Duration(0).to_string(), "0s");
        assert_eq!(Duration(3 * NANOS_PER_SECOND).to_string(), "3s");
        assert_eq!(Duration(1_000).to_string(), "0.000001s");
        assert_eq!(Duration(1_500_000_000).to_string(), "1.5s");
        assert_eq!(Duration(-1_000_000).to_string(), "-0.001s");
        assert_eq!(Duration(1).to_string(), "0.000000001s");
    }

    #[test]
    fn timestamps() {
        assert_eq!(Timestamp(0).to_string(), "1970-01-01T00:00:00Z");
        assert_eq!(
            Timestamp(1_500_000_000).to_string(),
            "1970-01-01T00:00:01.5Z"
        );
        assert_eq!(
            Timestamp(1_700_000_000 * NANOS_PER_SECOND).to_string(),
            "2023-11-14T22:13:20Z"
        );
        assert_eq!(
            Timestamp(-NANOS_PER_SECOND).to_string(),
            "1969-12-31T23:59:59Z"
        );
        assert_eq!(Timestamp::MIN.to_string(), "0001-01-01T00:00:00Z");
        assert_eq!(Timestamp::MAX.to_string(), "9999-12-31T23:59:59.999999999Z");
        // One nanosecond past the range: never handed to jiff.
        assert!(!Timestamp(Timestamp::MAX.0 + 1).in_range());
        assert_eq!(
            Timestamp(Timestamp::MAX.0 + 1).to_string(),
            "253402300800s since the epoch"
        );
        assert!(!Timestamp(Timestamp::MIN.0 - 1).in_range());
    }

    #[test]
    fn doubles() {
        assert_eq!(Double(1.5).to_string(), "1.5");
        assert_eq!(Double(123.0).to_string(), "123");
        assert_eq!(Double(1e21).to_string(), "1000000000000000000000");
        assert_eq!(Double(f64::INFINITY).to_string(), "Infinity");
        assert_eq!(Double(f64::NEG_INFINITY).to_string(), "-Infinity");
        assert_eq!(Double(f64::NAN).to_string(), "NaN");
        assert_eq!(
            List([f64::INFINITY, 1e21].map(Double)).to_string(),
            "[Infinity, 1000000000000000000000]"
        );
    }

    #[test]
    fn lists_and_bytes() {
        assert_eq!(List(&[1i64, 2]).to_string(), "[1, 2]");
        assert_eq!(List(&[] as &[i64]).to_string(), "[]");
        assert_eq!(List(&["foo", "bar"]).to_string(), "[foo, bar]");
        assert_eq!(List([456.789, 123.0]).to_string(), "[456.789, 123]");
        assert_eq!(
            List([b"a".as_slice(), b"b"].map(Text)).to_string(),
            "[a, b]"
        );
        assert_eq!(Hex(b"bar").to_string(), "626172");
        assert_eq!(Hex(&[0x99]).to_string(), "99");
    }
}
