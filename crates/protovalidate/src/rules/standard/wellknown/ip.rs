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

//! IPv4 and IPv6 addresses and prefixes, with the zone ID an IPv6 address
//! may carry and, for `strict` prefixes, host bits that must be zero.

use std::net::{Ipv4Addr, Ipv6Addr};

/// Returns whether the string is an IPv4 address in the dotted decimal format,
/// for example `192.168.5.21`.
pub(crate) fn is_ipv4(s: &str) -> bool {
    s.parse::<Ipv4Addr>().is_ok()
}

/// Returns whether the string is an IPv6 address in its text representation
/// following RFC 4291, for example `::1` or `2001:0DB8:ABCD:0012::0`, with an
/// optional zone ID following RFC 4007, for example `fe80::a%en1`.
///
/// There is no definition for the character set allowed in the zone
/// identifier. RFC 4007 permits basically any non-null string.
pub(crate) fn is_ipv6(s: &str) -> bool {
    let (address, zone) = match s.split_once('%') {
        Some((address, zone)) => (address, Some(zone)),
        None => (s, None),
    };
    if zone.is_some_and(|zone| zone.is_empty() || zone.contains('\0')) {
        return false;
    }
    address.parse::<Ipv6Addr>().is_ok()
}

/// Returns whether the string is an IPv4 or IPv6 address.
///
/// IPv4 addresses are expected in the dotted decimal format, for example
/// `192.168.5.21`. IPv6 addresses are expected in their text representation,
/// for example `::1`, or `2001:0DB8:ABCD:0012::0`.
///
/// Both formats are well-defined in the internet standard RFC 3986. Zone
/// identifiers for IPv6 addresses (for example `fe80::a%en1`) are supported.
pub(crate) fn is_ip(s: &str) -> bool {
    is_ipv4(s) || is_ipv6(s)
}

/// Parses the length of a prefix of an address of `bits` bits: decimal digits
/// with no sign and no leading zero, at most `bits`.
fn prefix_length(s: &str, bits: u32) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) || (s.len() > 1 && s.starts_with('0'))
    {
        return None;
    }
    s.parse::<u32>().ok().filter(|&length| length <= bits)
}

/// Returns whether the string is a valid IPv4 address prefix, optionally
/// requiring the host portion to be all zeros. See [`is_ip_prefix`].
pub(crate) fn is_ipv4_prefix(s: &str, strict: bool) -> bool {
    let Some((address, length)) = s.split_once('/') else {
        return false;
    };
    let (Ok(address), Some(length)) = (address.parse::<Ipv4Addr>(), prefix_length(length, 32))
    else {
        return false;
    };
    let host_mask = u32::MAX.checked_shr(length).unwrap_or(0);
    !strict || u32::from(address) & host_mask == 0
}

/// Returns whether the string is a valid IPv6 address prefix following RFC
/// 4291, optionally requiring the host portion to be all zeros. A zone ID is
/// not permitted. See [`is_ip_prefix`].
pub(crate) fn is_ipv6_prefix(s: &str, strict: bool) -> bool {
    let Some((address, length)) = s.split_once('/') else {
        return false;
    };
    let (Ok(address), Some(length)) = (address.parse::<Ipv6Addr>(), prefix_length(length, 128))
    else {
        return false;
    };
    let host_mask = u128::MAX.checked_shr(length).unwrap_or(0);
    !strict || u128::from(address) & host_mask == 0
}

/// Returns whether the string is a valid IP with prefix length, optionally
/// requiring the host portion to be all zeros.
///
/// An address prefix divides an IP address into a network portion, and a host
/// portion. The prefix length specifies how many bits the network portion has.
/// For example, the IPv6 prefix `2001:db8:abcd:0012::0/64` designates the
/// left-most 64 bits as the network prefix. The range of the network is 2**64
/// addresses, from `2001:db8:abcd:0012::0` to
/// `2001:db8:abcd:0012:ffff:ffff:ffff:ffff`.
///
/// An address prefix may include a specific host address, for example
/// `2001:db8:abcd:0012::1f/64`. With `strict` = true, this is not permitted.
/// The host portion must be all zeros, as in `2001:db8:abcd:0012::0/64`.
///
/// The same principle applies to IPv4 addresses. `192.168.1.0/24` designates
/// the first 24 bits of the 32-bit IPv4 as the network prefix.
pub(crate) fn is_ip_prefix(s: &str, strict: bool) -> bool {
    is_ipv4_prefix(s, strict) || is_ipv6_prefix(s, strict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4() {
        assert!(is_ipv4("192.168.0.1"));
        assert!(is_ipv4("0.0.0.0"));
        assert!(is_ipv4("255.255.255.255"));
        assert!(!is_ipv4("256.0.0.1"));
        assert!(!is_ipv4("01.0.0.1"));
        assert!(!is_ipv4("1.2.3"));
        assert!(!is_ipv4("1.2.3.4.5"));
        assert!(!is_ipv4(""));
    }

    #[test]
    fn ipv6() {
        assert!(is_ipv6("::"));
        assert!(is_ipv6("::1"));
        assert!(is_ipv6("2001:db8::1"));
        assert!(is_ipv6("2001:0db8:85a3:0000:0000:8a2e:0370:7334"));
        assert!(is_ipv6("::ffff:192.168.0.1"));
        assert!(is_ipv6("fe80::1%eth0"));
        assert!(!is_ipv6("fe80::1%"));
        assert!(!is_ipv6(":::"));
        assert!(!is_ipv6("1::2::3"));
        assert!(!is_ipv6("2001:db8:1:2:3:4:5:6:7"));
        assert!(!is_ipv6(":1"));
        assert!(!is_ipv6("1:"));
    }

    #[test]
    fn prefixes() {
        assert!(is_ipv4_prefix("192.168.0.0/16", true));
        assert!(!is_ipv4_prefix("192.168.0.1/16", true));
        assert!(is_ipv4_prefix("192.168.0.1/16", false));
        assert!(!is_ipv4_prefix("192.168.0.0/33", false));
        assert!(!is_ipv4_prefix("192.168.0.0/08", false));
        assert!(!is_ipv4_prefix("192.168.0.0/+8", false));
        assert!(is_ipv4_prefix("0.0.0.0/0", true));
        assert!(is_ipv6_prefix("2001:db8::/32", true));
        assert!(!is_ipv6_prefix("2001:db8::1/32", true));
        assert!(is_ipv6_prefix("::/0", true));
        assert!(is_ipv6_prefix("::1/128", true));
        assert!(!is_ipv6_prefix("2001:db8::/129", false));
        assert!(!is_ip_prefix("2001:db8::", false));
    }
}
