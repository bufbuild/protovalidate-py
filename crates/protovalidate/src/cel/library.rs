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

//! protovalidate's CEL function library: the functions custom rules can
//! call beyond CEL's own. Each entry of [`FUNCTIONS`] is one overload, named
//! and typed as CEL sees it.

use protovalidate_deps::{Arg, Element, Kind, NativeFn};

use crate::rules::standard::{UniqueKey, has_duplicates, wellknown};

/// The key `unique()` compares a list element by; `None` for elements that
/// equal nothing.
fn unique_key<'a>(element: &'a Element<'a>) -> Option<UniqueKey<'a>> {
    match element {
        Element::Bool(b) => Some(UniqueKey::Bool(*b)),
        Element::Int(i) => Some(UniqueKey::Int(*i)),
        Element::Uint(u) => Some(UniqueKey::Uint(*u)),
        Element::Double(f) => UniqueKey::double(*f),
        Element::String(s) => Some(UniqueKey::Str(s)),
        Element::Bytes(b) => Some(UniqueKey::Bytes(b)),
        Element::Other => None,
    }
}

/// One library function, called as a method of its first argument:
/// `arg0.name(args...)`.
pub(super) struct Function {
    pub name: &'static str,
    pub args: &'static [Kind],
    pub call: NativeFn,
}

/// Every function protovalidate adds to CEL, every overload a separate
/// entry. The backend registers each with its argument kinds, so a call
/// reaching `call` has arguments of exactly those kinds.
pub(super) static FUNCTIONS: &[Function] = &[
    Function {
        name: "isNan",
        args: &[Kind::Double],
        call: is_nan,
    },
    Function {
        name: "isInf",
        args: &[Kind::Double],
        call: is_inf,
    },
    Function {
        name: "isInf",
        args: &[Kind::Double, Kind::Int],
        call: is_inf,
    },
    Function {
        name: "unique",
        args: &[Kind::List],
        call: unique,
    },
    Function {
        name: "contains",
        args: &[Kind::Bytes, Kind::Bytes],
        call: contains,
    },
    Function {
        name: "startsWith",
        args: &[Kind::Bytes, Kind::Bytes],
        call: starts_with,
    },
    Function {
        name: "endsWith",
        args: &[Kind::Bytes, Kind::Bytes],
        call: ends_with,
    },
    Function {
        name: "isHostname",
        args: &[Kind::String],
        call: is_hostname,
    },
    Function {
        name: "isEmail",
        args: &[Kind::String],
        call: is_email,
    },
    Function {
        name: "isIp",
        args: &[Kind::String],
        call: is_ip,
    },
    Function {
        name: "isIp",
        args: &[Kind::String, Kind::Int],
        call: is_ip,
    },
    Function {
        name: "isIpPrefix",
        args: &[Kind::String],
        call: is_ip_prefix,
    },
    Function {
        name: "isIpPrefix",
        args: &[Kind::String, Kind::Int],
        call: is_ip_prefix,
    },
    Function {
        name: "isIpPrefix",
        args: &[Kind::String, Kind::Bool],
        call: is_ip_prefix,
    },
    Function {
        name: "isIpPrefix",
        args: &[Kind::String, Kind::Int, Kind::Bool],
        call: is_ip_prefix,
    },
    Function {
        name: "isUri",
        args: &[Kind::String],
        call: is_uri,
    },
    Function {
        name: "isUriRef",
        args: &[Kind::String],
        call: is_uri_ref,
    },
    Function {
        name: "isHostAndPort",
        args: &[Kind::String, Kind::Bool],
        call: is_host_and_port,
    },
];

// Below functions read the arguments passed from the C++ shim to
// pass to the wellknown functions that are also used for native rules.

fn mismatch(name: &str) -> String {
    format!("{name}: unexpected arguments")
}

fn is_nan(args: &[Arg<'_>]) -> Result<bool, String> {
    match args {
        [Arg::Double(value)] => Ok(value.is_nan()),
        _ => Err(mismatch("isNan")),
    }
}

/// `isInf()` for either infinity, `isInf(sign)` for the positive one when
/// `sign > 0`, the negative one when `sign < 0`, either when zero.
fn is_inf(args: &[Arg<'_>]) -> Result<bool, String> {
    let (value, sign) = match args {
        [Arg::Double(value)] => (*value, 0),
        [Arg::Double(value), Arg::Int(sign)] => (*value, *sign),
        _ => return Err(mismatch("isInf")),
    };
    Ok(value.is_infinite() && (sign == 0 || (sign > 0) == (value > 0.0)))
}

fn unique(args: &[Arg<'_>]) -> Result<bool, String> {
    match args {
        [Arg::List(elements)] => Ok(!has_duplicates(elements.iter().map(unique_key))),
        _ => Err(mismatch("unique")),
    }
}

fn bytes_pair<'a>(args: &'a [Arg<'_>], name: &str) -> Result<(&'a [u8], &'a [u8]), String> {
    match args {
        [Arg::Bytes(lhs), Arg::Bytes(rhs)] => Ok((lhs, rhs)),
        _ => Err(mismatch(name)),
    }
}

fn contains(args: &[Arg<'_>]) -> Result<bool, String> {
    let (haystack, needle) = bytes_pair(args, "contains")?;
    Ok(needle.is_empty()
        || haystack
            .windows(needle.len())
            .any(|window| window == needle))
}

fn starts_with(args: &[Arg<'_>]) -> Result<bool, String> {
    let (value, prefix) = bytes_pair(args, "startsWith")?;
    Ok(value.starts_with(prefix))
}

fn ends_with(args: &[Arg<'_>]) -> Result<bool, String> {
    let (value, suffix) = bytes_pair(args, "endsWith")?;
    Ok(value.ends_with(suffix))
}

fn string_arg<'a>(args: &'a [Arg<'_>], name: &str) -> Result<&'a str, String> {
    match args.first() {
        Some(Arg::String(value)) => Ok(value),
        _ => Err(mismatch(name)),
    }
}

fn is_hostname(args: &[Arg<'_>]) -> Result<bool, String> {
    Ok(wellknown::is_hostname(string_arg(args, "isHostname")?))
}

fn is_email(args: &[Arg<'_>]) -> Result<bool, String> {
    Ok(wellknown::is_email(string_arg(args, "isEmail")?))
}

/// `isIp()` for either version, `isIp(4)` or `isIp(6)` for one; any other
/// version is no IP address.
fn is_ip(args: &[Arg<'_>]) -> Result<bool, String> {
    let value = string_arg(args, "isIp")?;
    Ok(match args.get(1) {
        None | Some(Arg::Int(0)) => wellknown::is_ip(value),
        Some(Arg::Int(4)) => wellknown::is_ipv4(value),
        Some(Arg::Int(6)) => wellknown::is_ipv6(value),
        Some(Arg::Int(_)) => false,
        Some(_) => return Err(mismatch("isIp")),
    })
}

/// `isIpPrefix()`, `isIpPrefix(version)`, `isIpPrefix(strict)` or
/// `isIpPrefix(version, strict)`; strict also requires the host bits to be
/// zero.
fn is_ip_prefix(args: &[Arg<'_>]) -> Result<bool, String> {
    let value = string_arg(args, "isIpPrefix")?;
    let (version, strict) = match &args[1..] {
        [] => (0, false),
        [Arg::Int(version)] => (*version, false),
        [Arg::Bool(strict)] => (0, *strict),
        [Arg::Int(version), Arg::Bool(strict)] => (*version, *strict),
        _ => return Err(mismatch("isIpPrefix")),
    };
    Ok(match version {
        0 => wellknown::is_ip_prefix(value, strict),
        4 => wellknown::is_ipv4_prefix(value, strict),
        6 => wellknown::is_ipv6_prefix(value, strict),
        _ => false,
    })
}

fn is_uri(args: &[Arg<'_>]) -> Result<bool, String> {
    Ok(wellknown::is_uri(string_arg(args, "isUri")?))
}

fn is_uri_ref(args: &[Arg<'_>]) -> Result<bool, String> {
    Ok(wellknown::is_uri_ref(string_arg(args, "isUriRef")?))
}

fn is_host_and_port(args: &[Arg<'_>]) -> Result<bool, String> {
    match args {
        [Arg::String(value), Arg::Bool(port_required)] => {
            Ok(wellknown::is_host_and_port(value, *port_required))
        }
        _ => Err(mismatch("isHostAndPort")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinities() {
        let inf = |v: f64, sign: i64| is_inf(&[Arg::Double(v), Arg::Int(sign)]).unwrap();
        assert!(inf(f64::INFINITY, 0));
        assert!(inf(f64::INFINITY, 1));
        assert!(!inf(f64::INFINITY, -1));
        assert!(inf(f64::NEG_INFINITY, -1));
        assert!(!inf(f64::NEG_INFINITY, 1));
        assert!(!inf(1.0, 0));
        assert!(is_inf(&[Arg::Double(f64::NEG_INFINITY)]).unwrap());
    }

    #[test]
    fn uniqueness() {
        let list = |elements| unique(&[Arg::List(elements)]).unwrap();
        assert!(list(vec![Element::Int(1), Element::Int(2)]));
        assert!(!list(vec![Element::Int(1), Element::Int(1)]));
        assert!(list(vec![
            Element::Double(f64::NAN),
            Element::Double(f64::NAN)
        ]));
        assert!(!list(vec![Element::Double(0.0), Element::Double(-0.0)]));
        assert!(list(vec![Element::Other, Element::Other]));
        assert!(!list(vec![
            Element::String("a".into()),
            Element::String("a".into())
        ]));
    }

    #[test]
    fn overload_shapes_match_registrations() {
        // Every entry's `call` accepts arguments of the kinds it declares.
        for function in FUNCTIONS {
            let args: Vec<Arg<'_>> = function
                .args
                .iter()
                .map(|kind| match kind {
                    Kind::Bool => Arg::Bool(false),
                    Kind::Int => Arg::Int(0),
                    Kind::Uint => Arg::Uint(0),
                    Kind::Double => Arg::Double(0.0),
                    Kind::String => Arg::String("".into()),
                    Kind::Bytes => Arg::Bytes(b""),
                    Kind::List => Arg::List(Vec::new()),
                })
                .collect();
            assert!((function.call)(&args).is_ok(), "{}", function.name);
        }
    }
}
