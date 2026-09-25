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

//! The formats `validate.proto` defines with a regular expression: UUIDs,
//! ULIDs, Protobuf names and HTTP header names and values.

use std::sync::LazyLock;

use regex::Regex;

macro_rules! pattern {
    ($name:ident, $pattern:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new($pattern).expect(concat!("`", stringify!($name), "` is a valid pattern"))
        });
    };
}

pattern!(
    UUID,
    r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
);
pattern!(TUUID, r"^[0-9a-fA-F]{32}$");
pattern!(ULID, r"^[0-7][0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{25}$");
pattern!(
    PROTOBUF_FQN,
    r"^[A-Za-z_][A-Za-z_0-9]*(\.[A-Za-z_][A-Za-z_0-9]*)*$"
);
pattern!(
    PROTOBUF_DOT_FQN,
    r"^\.[A-Za-z_][A-Za-z_0-9]*(\.[A-Za-z_][A-Za-z_0-9]*)*$"
);
// RFC 7230's token, with a leading colon allowed for HTTP/2 pseudo-headers.
pattern!(HEADER_NAME_STRICT, r"^:?[0-9a-zA-Z!#$%&'*+-.^_|~\x60]+$");
pattern!(HEADER_NAME_LOOSE, r"^[^\x00\x0A\x0D]+$");
// Everything but the control characters other than tab.
pattern!(HEADER_VALUE_STRICT, r"^[^\x00-\x08\x0A-\x1F\x7F]*$");
pattern!(HEADER_VALUE_LOOSE, r"^[^\x00\x0A\x0D]*$");

pub(crate) fn is_uuid(s: &str) -> bool {
    UUID.is_match(s)
}

pub(crate) fn is_tuuid(s: &str) -> bool {
    TUUID.is_match(s)
}

pub(crate) fn is_ulid(s: &str) -> bool {
    ULID.is_match(s)
}

pub(crate) fn is_protobuf_fqn(s: &str) -> bool {
    PROTOBUF_FQN.is_match(s)
}

pub(crate) fn is_protobuf_dot_fqn(s: &str) -> bool {
    PROTOBUF_DOT_FQN.is_match(s)
}

/// An HTTP header name; loose only excludes NUL, LF and CR.
pub(crate) fn is_header_name(s: &str, strict: bool) -> bool {
    if strict {
        HEADER_NAME_STRICT.is_match(s)
    } else {
        HEADER_NAME_LOOSE.is_match(s)
    }
}

/// An HTTP header value; loose only excludes NUL, LF and CR.
pub(crate) fn is_header_value(s: &str, strict: bool) -> bool {
    if strict {
        HEADER_VALUE_STRICT.is_match(s)
    } else {
        HEADER_VALUE_LOOSE.is_match(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuids() {
        assert!(is_uuid("123e4567-e89b-12d3-a456-426614174000"));
        assert!(is_uuid("123E4567-E89B-12D3-A456-426614174000"));
        assert!(!is_uuid("123e4567e89b12d3a456426614174000"));
        assert!(!is_uuid("123e4567-e89b-12d3-a456-42661417400g"));
        assert!(is_tuuid("123e4567e89b12d3a456426614174000"));
        assert!(!is_tuuid("123e4567-e89b-12d3-a456-426614174000"));
    }

    #[test]
    fn ulids() {
        assert!(is_ulid("01ARZ3NDEKTSV4RRFFQ69G5FAV"));
        assert!(is_ulid("01arz3ndektsv4rrffq69g5fav"));
        assert!(!is_ulid("81ARZ3NDEKTSV4RRFFQ69G5FAV"));
        assert!(!is_ulid("01ARZ3NDEKTSV4RRFFQ69G5FAI"));
        assert!(!is_ulid("01ARZ3NDEKTSV4RRFFQ69G5FA"));
    }

    #[test]
    fn protobuf_names() {
        assert!(is_protobuf_fqn("foo.bar.Baz"));
        assert!(is_protobuf_fqn("_x1"));
        assert!(!is_protobuf_fqn("foo..Baz"));
        assert!(!is_protobuf_fqn("1foo"));
        assert!(!is_protobuf_fqn(".foo"));
        assert!(is_protobuf_dot_fqn(".foo.Bar"));
        assert!(!is_protobuf_dot_fqn("foo.Bar"));
    }

    #[test]
    fn headers() {
        assert!(is_header_name("Content-Type", true));
        assert!(is_header_name(":authority", true));
        assert!(!is_header_name("Content Type", true));
        assert!(is_header_name("Content Type", false));
        assert!(!is_header_name("Content\nType", false));
        assert!(!is_header_name("", true));
        assert!(is_header_value("text/html; charset=utf-8", true));
        assert!(is_header_value("tab\tok", true));
        assert!(is_header_value("", true));
        assert!(!is_header_value("bad\x7f", true));
        assert!(is_header_value("bad\x7f", false));
        assert!(!is_header_value("bad\r", false));
    }
}
