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

//! The regular expression engine behind `pattern` rules and the well-known
//! string formats.
//!
//! With the `cel` feature this is RE2, through cel-cpp, so a `pattern`
//! rule and a `matches()` call in a custom rule use the same engine.
//! Without it, the `regex` crate is used, after [`ascii_classes`] rewrites
//! the Perl classes to the ASCII sets RE2 uses. The remaining differences
//! from RE2 are that `\Q...\E` is not supported, repeat counts are not
//! capped at 1000, and more `\p{...}` names are accepted.

#[cfg(feature = "cel")]
type Inner = protovalidate_deps::Regex;
#[cfg(not(feature = "cel"))]
type Inner = regex::Regex;

/// A compiled pattern.
#[derive(Debug)]
pub(crate) struct Regex(Inner);

impl Regex {
    /// Compiles `pattern`, or returns the engine's message for one it
    /// rejects.
    pub(crate) fn new(pattern: &str) -> Result<Self, String> {
        #[cfg(not(feature = "cel"))]
        let pattern = &ascii_classes(pattern);
        Inner::new(pattern)
            .map(Self)
            .map_err(|error| error.to_string())
    }

    /// Whether `text` contains a match.
    pub(crate) fn is_match(&self, text: &str) -> bool {
        self.0.is_match(text)
    }
}

/// Rewrites `\d`, `\w`, `\s`, `\b` and their negations to the ASCII sets
/// RE2 uses. In the `regex` crate they match Unicode. `\s` is written out
/// as a class because the crate's ASCII `\s` includes vertical tab and
/// RE2's does not. Inside a bracket class the set is inserted directly,
/// or as a nested class when negated.
///
/// The pattern is scanned, not parsed. The scan handles escapes, so `\\d`
/// stays a backslash followed by `d`, and bracket depth, so a `]` directly
/// after `[` or `[^` is treated as a literal. Everything else is copied
/// through, so a pattern the crate rejects is still rejected afterwards.
#[cfg(any(not(feature = "cel"), test))]
fn ascii_classes(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    let mut depth = 0usize;
    // A `]` directly after `[` or `[^` is a literal, not the end of the
    // class.
    let mut class_start = false;
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                class_start = false;
                let Some(escaped) = chars.next() else {
                    out.push('\\');
                    break;
                };
                let in_class = depth > 0;
                let replacement = match (escaped, in_class) {
                    ('d', false) => "[0-9]",
                    ('d', true) => "0-9",
                    ('D', _) => "[^0-9]",
                    ('w', false) => "[0-9A-Za-z_]",
                    ('w', true) => "0-9A-Za-z_",
                    ('W', _) => "[^0-9A-Za-z_]",
                    ('s', false) => "[\\t\\n\\f\\r ]",
                    ('s', true) => "\\t\\n\\f\\r ",
                    ('S', _) => "[^\\t\\n\\f\\r ]",
                    ('b', false) => "(?-u:\\b)",
                    ('B', false) => "(?-u:\\B)",
                    _ => {
                        out.push('\\');
                        out.push(escaped);
                        continue;
                    }
                };
                out.push_str(replacement);
            }
            '[' => {
                depth += 1;
                out.push('[');
                if chars.peek() == Some(&'^') {
                    chars.next();
                    out.push('^');
                }
                class_start = true;
            }
            ']' if class_start => {
                out.push(']');
                class_start = false;
            }
            ']' => {
                depth = depth.saturating_sub(1);
                out.push(']');
            }
            _ => {
                class_start = false;
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::ascii_classes;

    #[test]
    fn rewrites_perl_classes() {
        assert_eq!(ascii_classes(r"^\d+$"), "^[0-9]+$");
        assert_eq!(
            ascii_classes(r"\D\w\W\S"),
            "[^0-9][0-9A-Za-z_][^0-9A-Za-z_][^\\t\\n\\f\\r ]"
        );
        assert_eq!(ascii_classes(r"\bfoo\B"), r"(?-u:\b)foo(?-u:\B)");
        assert_eq!(ascii_classes(r"\s"), "[\\t\\n\\f\\r ]");
    }

    #[test]
    fn splices_into_classes() {
        assert_eq!(ascii_classes(r"[\d-]"), "[0-9-]");
        assert_eq!(ascii_classes(r"[^\w\s]"), "[^0-9A-Za-z_\\t\\n\\f\\r ]");
        assert_eq!(ascii_classes(r"[a\D]"), "[a[^0-9]]");
        assert_eq!(ascii_classes(r"[[:alpha:]\d]"), "[[:alpha:]0-9]");
        assert_eq!(ascii_classes(r"[[:alpha:]]\d"), "[[:alpha:]][0-9]");
    }

    #[test]
    fn leaves_the_rest_alone() {
        assert_eq!(ascii_classes(r"\\d"), r"\\d");
        assert_eq!(ascii_classes(r"[]\d]"), "[]0-9]");
        assert_eq!(ascii_classes(r"[^]\d]"), "[^]0-9]");
        assert_eq!(ascii_classes(r"[\b]"), r"[\b]");
        assert_eq!(ascii_classes(r"\p{Greek}\x41é"), r"\p{Greek}\x41é");
        assert_eq!(ascii_classes(r"a\"), r"a\");
    }

    #[test]
    fn matches_like_re2() {
        let re = |p: &str| regex::Regex::new(&ascii_classes(p)).expect("valid");
        assert!(re(r"^\d+$").is_match("123"));
        assert!(!re(r"^\d+$").is_match("١٢٣"));
        assert!(!re(r"^\w+$").is_match("héllo"));
        assert!(re(r"^[^\d]$").is_match("٣"));
        assert!(re(r"^[a\D]$").is_match("٣"));
        assert!(re(r"\bfoo\b").is_match("üfoo"));
        assert!(!re(r"^\s$").is_match("\u{a0}"));
        assert!(!re(r"^\s$").is_match("\u{b}"));
        assert!(re(r"^.$").is_match("é"));
        assert!(re(r"^[^a]$").is_match("é"));
        assert!(re(r"(?i)σ").is_match("Σ"));
        assert!(re(r"(?i)\w").is_match("\u{212a}"));
    }
}
