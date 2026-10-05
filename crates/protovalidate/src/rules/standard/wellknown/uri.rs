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

//! URIs and URI references per RFC 3986, with two additions: an IPv6
//! literal may carry an RFC 6874 zone ID, `[fe80::1%25eth0]`, and the
//! percent-encoded octets of a host name or zone ID must form valid UTF-8.

use std::borrow::Cow;

use iri_string::types::{UriReferenceStr, UriStr};
use percent_encoding::percent_decode_str;

/// Whether the percent-encoded octets of `text`, whose triplets are already
/// known to be well-formed, decode to UTF-8.
fn decodes_to_utf8(text: &str) -> bool {
    percent_decode_str(text).decode_utf8().is_ok()
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~')
}

/// The rule from RFC 6874, with the encoded octets valid UTF-8:
///
/// ```text
/// ZoneID = 1*( unreserved / pct-encoded )
/// ```
fn is_zone_id(zone: &str) -> bool {
    let mut rest = zone.as_bytes();
    if rest.is_empty() {
        return false;
    }
    while let Some(&b) = rest.first() {
        rest = match b {
            b'%' if rest.len() >= 3 && rest[1..3].iter().all(u8::is_ascii_hexdigit) => &rest[3..],
            _ if is_unreserved(b) => &rest[1..],
            _ => return false,
        };
    }
    decodes_to_utf8(zone)
}

/// `s` with the zone ID of its IPv6 literal removed, for RFC 3986's grammar,
/// once the zone ID has been checked; `None` for a bad zone ID.
///
/// A raw `[` is only legal in an IP literal, so the first one is the host's
/// if the URI is valid at all, and taking a zone ID out of a `[...]` anywhere
/// else changes nothing about the string being rejected.
fn without_zone_id(s: &str) -> Option<Cow<'_, str>> {
    let Some(open) = s.find('[') else {
        return Some(Cow::Borrowed(s));
    };
    let close = open + s[open..].find(']')?;
    let Some((address, zone)) = s[open + 1..close].split_once("%25") else {
        return Some(Cow::Borrowed(s));
    };
    is_zone_id(zone).then(|| Cow::Owned(format!("{}{address}{}", &s[..=open], &s[close..])))
}

/// A registered name's percent-encoded octets must be UTF-8; IP literals
/// are in brackets.
fn is_host_utf8(host: &str) -> bool {
    host.starts_with('[') || decodes_to_utf8(host)
}

/// Returns whether the string is a URI, for example
/// `https://example.com/foo/bar?baz=quux#frag`.
///
/// URI is defined in the internet standard RFC 3986.
/// Zone Identifiers in IPv6 address literals are supported (RFC 6874).
pub(crate) fn is_uri(s: &str) -> bool {
    let Some(s) = without_zone_id(s) else {
        return false;
    };
    UriStr::new(&s).is_ok_and(|uri| {
        uri.authority_components()
            .is_none_or(|authority| is_host_utf8(authority.host()))
    })
}

/// Returns whether the string is a URI Reference - a URI such as
/// `https://example.com/foo/bar?baz=quux#frag`, or a Relative Reference such
/// as `./foo/bar?query`.
///
/// URI, URI Reference, and Relative Reference are defined in the internet
/// standard RFC 3986. Zone Identifiers in IPv6 address literals are supported
/// (RFC 6874).
pub(crate) fn is_uri_ref(s: &str) -> bool {
    let Some(s) = without_zone_id(s) else {
        return false;
    };
    UriReferenceStr::new(&s).is_ok_and(|uri| {
        uri.authority_components()
            .is_none_or(|authority| is_host_utf8(authority.host()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris() {
        assert!(is_uri("https://example.com"));
        assert!(is_uri("https://user:pw@example.com:8080/a/b?c=d#e"));
        assert!(is_uri("mailto:someone@example.com"));
        assert!(is_uri("urn:isbn:0451450523"));
        assert!(is_uri("http://[2001:db8::1]/"));
        assert!(is_uri("http://[fe80::1%25eth0]/"));
        assert!(is_uri("http://[fe80::1%25foo%c3%96]/"));
        assert!(is_uri("http://[v1.fe80::a+en1]/"));
        assert!(is_uri("http://ex%C3%A4mple.com/"));
        assert!(!is_uri("http://ex%C3mple.com/"));
        assert!(!is_uri("http://[fe80::1%25]/"));
        assert!(!is_uri("http://[fe80::1%25a%c3]/"));
        assert!(!is_uri("http://[fe80::1%eth0]/"));
        assert!(!is_uri("example.com"));
        assert!(!is_uri("http://example.com/a b"));
        assert!(!is_uri(""));
        assert!(!is_uri("1http://example.com"));
    }

    #[test]
    fn uri_refs() {
        assert!(is_uri_ref("https://example.com"));
        assert!(is_uri_ref("/relative/path?x=y"));
        assert!(is_uri_ref("relative/path"));
        assert!(is_uri_ref("//example.com/path"));
        assert!(is_uri_ref(""));
        assert!(is_uri_ref("#frag"));
        assert!(!is_uri_ref("a:b:c/ d"));
        assert!(!is_uri_ref("%"));
    }
}
