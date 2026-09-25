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
//! Doubles print in the shortest round-trip form, and fractions of a second
//! with the digits they need.

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

/// Bytes as `%s` prints them: as text, with what is not UTF-8 replaced.
#[derive(Clone, Copy)]
pub(crate) struct Text<'a>(pub &'a [u8]);

/// Bytes as `%x` prints them: lowercase hex.
#[derive(Clone, Copy)]
pub(crate) struct Hex<'a>(pub &'a [u8]);

/// A list as `%s` prints it: the elements, comma separated, in brackets.
pub(crate) struct List<I>(pub I);

/// A fraction of a second, in the digits it needs; nothing for zero.
fn fraction(f: &mut fmt::Formatter<'_>, nanos: i128) -> fmt::Result {
    if nanos == 0 {
        return Ok(());
    }
    let digits = format!("{nanos:09}");
    write!(f, ".{}", digits.trim_end_matches('0'))
}

/// `1.5s`, `-0.000001s`, `0s`.
impl Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let magnitude = self.0.unsigned_abs();
        let seconds = magnitude / NANOS_PER_SECOND.unsigned_abs();
        let nanos = magnitude % NANOS_PER_SECOND.unsigned_abs();
        if self.0 < 0 {
            f.write_str("-")?;
        }
        write!(f, "{seconds}")?;
        fraction(f, i128::try_from(nanos).expect("under a second"))?;
        f.write_str("s")
    }
}

/// RFC 3339 in UTC, with the fractional digits the value needs. A value
/// outside the years jiff represents prints as seconds since the epoch.
impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match jiff::Timestamp::from_nanosecond(self.0) {
            Ok(timestamp) => write!(f, "{timestamp}"),
            Err(_) => write!(f, "{} since the epoch", Duration(self.0)),
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
