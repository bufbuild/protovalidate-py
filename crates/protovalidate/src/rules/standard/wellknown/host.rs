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

//! Hostnames, email addresses and host-and-port pairs.

use std::sync::LazyLock;

use regex::Regex;

use super::ip::{is_ipv4, is_ipv6};

/// Returns whether the string is a valid hostname, for example
/// `foo.example.com`.
///
/// A valid hostname follows the rules below:
/// - The name consists of one or more labels, separated by a dot (`.`).
/// - Each label can be 1 to 63 alphanumeric characters.
/// - A label can contain hyphens (`-`), but must not start or end with a hyphen.
/// - The right-most label must not be digits only.
/// - The name can have a trailing dot, for example `foo.example.com.`.
/// - The name can be 253 characters at most, excluding the optional trailing dot.
pub(crate) fn is_hostname(s: &str) -> bool {
    if s.is_empty() || s.len() > 253 {
        return false;
    }
    let s = s.strip_suffix('.').unwrap_or(s);
    let mut all_digits = false;
    for part in s.split('.') {
        all_digits = true;
        let bytes = part.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 63
            || bytes[0] == b'-'
            || bytes[bytes.len() - 1] == b'-'
        {
            return false;
        }
        for &b in bytes {
            if !b.is_ascii_alphanumeric() && b != b'-' {
                return false;
            }
            all_digits = all_digits && b.is_ascii_digit();
        }
    }
    !all_digits
}

/// Returns whether val is an email address, for example "foo@example.com".
///
/// Conforms to the definition for a valid email address from the HTML standard.
/// Note that this standard willfully deviates from RFC 5322, which allows many
/// unexpected forms of email addresses and will easily match a typographical
/// error.
pub(crate) fn is_email(s: &str) -> bool {
    static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
        // See https://html.spec.whatwg.org/multipage/input.html#valid-e-mail-address
        Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$",
        )
        .expect("the email pattern is valid")
    });
    EMAIL.is_match(s)
}

/// Returns whether the string is a valid port for [`is_host_and_port`].
fn is_port(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() || (bytes.len() > 1 && bytes[0] == b'0') {
        return false;
    }
    if !bytes.iter().all(u8::is_ascii_digit) {
        return false;
    }
    s.parse::<u32>().is_ok_and(|port| port <= 65_535)
}

/// Returns whether the string is a valid host/port pair, for example
/// `example.com:8080`.
///
/// If the argument `port_required` is true, the port is required. If the
/// argument is false, the port is optional.
///
/// The host can be one of:
/// - An IPv4 address in dotted decimal format, for example `192.168.0.1`.
/// - An IPv6 address enclosed in square brackets, for example `[::1]`.
/// - A hostname, for example `example.com`.
///
/// The port is separated by a colon. It must be non-empty, with a decimal
/// number in the range of 0-65535, inclusive.
pub(crate) fn is_host_and_port(s: &str, port_required: bool) -> bool {
    if s.is_empty() {
        return false;
    }
    let split = s.rfind(':');
    if s.starts_with('[') {
        let Some(end) = s.rfind(']') else {
            return false;
        };
        let after_end = end + 1;
        if after_end == s.len() {
            // No port.
            return !port_required && is_ipv6(&s[1..end]);
        }
        if Some(after_end) == split {
            return is_ipv6(&s[1..end]) && is_port(&s[after_end + 1..]);
        }
        return false;
    }
    let Some(split) = split else {
        return !port_required && (is_hostname(s) || is_ipv4(s));
    };
    let (host, port) = (&s[..split], &s[split + 1..]);
    (is_hostname(host) || is_ipv4(host)) && is_port(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames() {
        assert!(is_hostname("example.com"));
        assert!(is_hostname("example.com."));
        assert!(is_hostname("a-b.c"));
        assert!(is_hostname("123.example"));
        assert!(!is_hostname("example..com"));
        assert!(!is_hostname("-example.com"));
        assert!(!is_hostname("example.com.."));
        assert!(!is_hostname("192.168.0.1"));
        assert!(!is_hostname(""));
        assert!(!is_hostname("."));
        assert!(!is_hostname("exa_mple.com"));
    }

    #[test]
    fn emails() {
        assert!(is_email("foo@example.com"));
        assert!(is_email("a.b+c@sub.example.com"));
        assert!(!is_email("foo@"));
        assert!(!is_email("@example.com"));
        assert!(!is_email("foo@-example.com"));
        assert!(!is_email("foo@example..com"));
        assert!(!is_email("foo bar@example.com"));
        assert!(!is_email("foo@exa_mple.com"));
    }

    #[test]
    fn host_and_port() {
        assert!(is_host_and_port("example.com:80", true));
        assert!(is_host_and_port("192.168.0.1:8080", true));
        assert!(is_host_and_port("[::1]:443", true));
        assert!(!is_host_and_port("example.com", true));
        assert!(is_host_and_port("example.com", false));
        assert!(is_host_and_port("[::1]", false));
        assert!(!is_host_and_port("example.com:0080", true));
        assert!(!is_host_and_port("example.com:65536", true));
        assert!(!is_host_and_port("::1:443", true));
        assert!(!is_host_and_port("", false));
    }
}
