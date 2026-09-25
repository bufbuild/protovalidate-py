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

//! The string rules.

use std::mem;

use super::{Unplaced, regex};
use crate::rules::standard::format::List;
use crate::rules::standard::{StrTest, WellKnown};
use crate::validate::__buffa::oneof;
use crate::validate::{KnownRegex, StringRules};

pub(super) fn checks<'a>(
    prefix: &'a str,
    r: &mut StringRules,
) -> Result<Unplaced<'a, StrTest>, String> {
    let mut checks = Unplaced::new(prefix);
    if let Some(c) = r.r#const.take() {
        checks.push(1, "const", format!("must equal `{c}`"), StrTest::Const(c));
    }
    length_checks(&mut checks, r);
    if let Some(p) = r.pattern.take() {
        let compiled = regex(&p)?;
        checks.push(
            6,
            "pattern",
            format!("does not match regex pattern `{p}`"),
            StrTest::Pattern(compiled),
        );
    }
    substring_checks(&mut checks, r);
    if !r.r#in.is_empty() {
        let list = mem::take(&mut r.r#in);
        checks.push(
            10,
            "in",
            format!("must be in list {}", List(&list)),
            StrTest::In(list),
        );
    }
    if !r.not_in.is_empty() {
        let list = mem::take(&mut r.not_in);
        checks.push(
            11,
            "not_in",
            format!("must not be in list {}", List(&list)),
            StrTest::NotIn(list),
        );
    }
    // `strict` qualifies `well_known_regex`, and is read with it.
    let strict = r.strict.take().unwrap_or(true);
    if let Some(rule) = r.well_known.take() {
        well_known_checks(&mut checks, &rule, strict);
    }
    Ok(checks)
}

/// The length rules.
fn length_checks(checks: &mut Unplaced<'_, StrTest>, r: &mut StringRules) {
    if let Some(n) = r.len.take() {
        checks.push(
            19,
            "len",
            format!("must be {n} characters"),
            StrTest::Len(n),
        );
    }
    if let Some(n) = r.min_len.take() {
        checks.push(
            2,
            "min_len",
            format!("must be at least {n} characters"),
            StrTest::MinLen(n),
        );
    }
    if let Some(n) = r.max_len.take() {
        checks.push(
            3,
            "max_len",
            format!("must be at most {n} characters"),
            StrTest::MaxLen(n),
        );
    }
    if let Some(n) = r.len_bytes.take() {
        checks.push(
            20,
            "len_bytes",
            format!("must be {n} bytes"),
            StrTest::LenBytes(n),
        );
    }
    if let Some(n) = r.min_bytes.take() {
        checks.push(
            4,
            "min_bytes",
            format!("must be at least {n} bytes"),
            StrTest::MinBytes(n),
        );
    }
    if let Some(n) = r.max_bytes.take() {
        checks.push(
            5,
            "max_bytes",
            format!("must be at most {n} bytes"),
            StrTest::MaxBytes(n),
        );
    }
}

/// The substring rules.
fn substring_checks(checks: &mut Unplaced<'_, StrTest>, r: &mut StringRules) {
    if let Some(p) = r.prefix.take() {
        checks.push(
            7,
            "prefix",
            format!("does not have prefix `{p}`"),
            StrTest::Prefix(p),
        );
    }
    if let Some(p) = r.suffix.take() {
        checks.push(
            8,
            "suffix",
            format!("does not have suffix `{p}`"),
            StrTest::Suffix(p),
        );
    }
    if let Some(p) = r.contains.take() {
        checks.push(
            9,
            "contains",
            format!("does not contain substring `{p}`"),
            StrTest::Contains(p),
        );
    }
    if let Some(p) = r.not_contains.take() {
        checks.push(
            23,
            "not_contains",
            format!("contains substring `{p}`"),
            StrTest::NotContains(p),
        );
    }
}

/// The checks of a well-known format rule: the format check itself, and
/// for a format that lets an empty string through, an `_empty` companion
/// naming what an empty string is not.
fn well_known_checks(
    checks: &mut Unplaced<'_, StrTest>,
    rule: &oneof::string_rules::WellKnown,
    strict: bool,
) {
    let Some((number, suffix, what, format)) = well_known_format(rule, strict) else {
        return;
    };
    let allows_empty = format.allows_empty();
    // The one companion whose message says it differently.
    let empty = match format {
        WellKnown::HostAndPort => "host and port pair",
        _ => what,
    };
    checks.push(
        number,
        suffix,
        format!("must be a valid {what}"),
        StrTest::WellKnown(format),
    );
    if allows_empty {
        checks.push(
            number,
            &format!("{suffix}_empty"),
            format!("value is empty, which is not a valid {empty}"),
            StrTest::Empty,
        );
    }
}

/// An enabled well-known format rule: its field number, rule id suffix,
/// what a value must be, and the format.
type Format = (u32, &'static str, &'static str, WellKnown);

/// The [`Format`] of a well-known format rule, `None` for a format flag set
/// to false or a `well_known_regex` naming no format.
fn well_known_format(rule: &oneof::string_rules::WellKnown, strict: bool) -> Option<Format> {
    use oneof::string_rules::WellKnown as Wk;
    let (enabled, number, suffix, what, format) = match rule {
        Wk::Email(b) => (*b, 12, "email", "email address", WellKnown::Email),
        Wk::Hostname(b) => (*b, 13, "hostname", "hostname", WellKnown::Hostname),
        Wk::Ip(b) => (*b, 14, "ip", "IP address", WellKnown::Ip),
        Wk::Ipv4(b) => (*b, 15, "ipv4", "IPv4 address", WellKnown::Ipv4),
        Wk::Ipv6(b) => (*b, 16, "ipv6", "IPv6 address", WellKnown::Ipv6),
        Wk::Uri(b) => (*b, 17, "uri", "URI", WellKnown::Uri),
        Wk::UriRef(b) => (*b, 18, "uri_ref", "URI Reference", WellKnown::UriRef),
        Wk::Address(b) => (
            *b,
            21,
            "address",
            "hostname, or ip address",
            WellKnown::Address,
        ),
        Wk::Uuid(b) => (*b, 22, "uuid", "UUID", WellKnown::Uuid),
        Wk::WellKnownRegex(regex) => {
            let (suffix, what, format) = match regex {
                KnownRegex::KNOWN_REGEX_HTTP_HEADER_NAME => (
                    "well_known_regex.header_name",
                    "HTTP header name",
                    WellKnown::HeaderName { strict },
                ),
                KnownRegex::KNOWN_REGEX_HTTP_HEADER_VALUE => (
                    "well_known_regex.header_value",
                    "HTTP header value",
                    WellKnown::HeaderValue { strict },
                ),
                KnownRegex::KNOWN_REGEX_UNSPECIFIED => return None,
            };
            (true, 24, suffix, what, format)
        }
        Wk::IpWithPrefixlen(b) => (
            *b,
            26,
            "ip_with_prefixlen",
            "IP prefix",
            WellKnown::IpWithPrefixlen,
        ),
        Wk::Ipv4WithPrefixlen(b) => (
            *b,
            27,
            "ipv4_with_prefixlen",
            "IPv4 address with prefix length",
            WellKnown::Ipv4WithPrefixlen,
        ),
        Wk::Ipv6WithPrefixlen(b) => (
            *b,
            28,
            "ipv6_with_prefixlen",
            "IPv6 address with prefix length",
            WellKnown::Ipv6WithPrefixlen,
        ),
        Wk::IpPrefix(b) => (*b, 29, "ip_prefix", "IP prefix", WellKnown::IpPrefix),
        Wk::Ipv4Prefix(b) => (*b, 30, "ipv4_prefix", "IPv4 prefix", WellKnown::Ipv4Prefix),
        Wk::Ipv6Prefix(b) => (*b, 31, "ipv6_prefix", "IPv6 prefix", WellKnown::Ipv6Prefix),
        Wk::HostAndPort(b) => (
            *b,
            32,
            "host_and_port",
            "host (hostname or IP address) and port pair",
            WellKnown::HostAndPort,
        ),
        Wk::Tuuid(b) => (*b, 33, "tuuid", "trimmed UUID", WellKnown::Tuuid),
        Wk::Ulid(b) => (*b, 35, "ulid", "ULID", WellKnown::Ulid),
        Wk::ProtobufFqn(b) => (
            *b,
            37,
            "protobuf_fqn",
            "fully-qualified Protobuf name",
            WellKnown::ProtobufFqn,
        ),
        Wk::ProtobufDotFqn(b) => (
            *b,
            38,
            "protobuf_dot_fqn",
            "fully-qualified Protobuf name with a leading dot",
            WellKnown::ProtobufDotFqn,
        ),
    };
    enabled.then_some((number, suffix, what, format))
}
