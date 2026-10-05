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

//! Regular expressions through RE2, the engine behind cel-cpp's `matches()`,
//! so that patterns compiled here and patterns in CEL expressions share one
//! syntax and one set of limits.

use std::ffi::{c_char, c_int};
use std::fmt;
use std::ptr;

use crate::Error;
use crate::engine::finish;

#[repr(C)]
struct Re2Regex {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    fn re2_regex_new(
        pattern: *const c_char,
        len: usize,
        out: *mut *mut Re2Regex,
        error: *mut *mut c_char,
    ) -> c_int;
    fn re2_regex_free(regex: *mut Re2Regex);
    fn re2_regex_matches(regex: *const Re2Regex, text: *const c_char, len: usize) -> c_int;
}

/// A compiled regular expression, in RE2 syntax.
pub struct Regex {
    raw: *mut Re2Regex,
}

// SAFETY: a compiled RE2 is immutable, and RE2 documents matching as safe
// to call from several threads at once.
unsafe impl Send for Regex {}
// SAFETY: as above.
unsafe impl Sync for Regex {}

impl Regex {
    /// Compiles `pattern`.
    ///
    /// # Errors
    ///
    /// [`Error::Compilation`] with RE2's message for a pattern it rejects.
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let mut out: *mut Re2Regex = ptr::null_mut();
        let mut error: *mut c_char = ptr::null_mut();
        // SAFETY: the pattern is valid for its length for the call, which
        // copies it; `out` and `error` are live out-params.
        let code = unsafe {
            re2_regex_new(
                pattern.as_ptr().cast::<c_char>(),
                pattern.len(),
                &raw mut out,
                &raw mut error,
            )
        };
        // SAFETY: `code` and `error` are the call's, untouched since.
        unsafe { finish(code, error) }?;
        Ok(Self { raw: out })
    }

    /// Whether `text` contains a match, anchored only where the pattern
    /// anchors itself.
    #[must_use]
    pub fn is_match(&self, text: &str) -> bool {
        // SAFETY: `self.raw` is a live regex, and the text is valid for its
        // length for the call.
        unsafe { re2_regex_matches(self.raw, text.as_ptr().cast::<c_char>(), text.len()) != 0 }
    }
}

impl fmt::Debug for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Regex").finish_non_exhaustive()
    }
}

impl Drop for Regex {
    fn drop(&mut self) {
        // SAFETY: `self.raw` came from `re2_regex_new` and is freed once.
        unsafe { re2_regex_free(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::Regex;
    use crate::Error;

    #[test]
    fn matches_partially() {
        let regex = Regex::new("b+").expect("valid");
        assert!(regex.is_match("abbc"));
        assert!(!regex.is_match("ac"));
        assert!(Regex::new("^é$").expect("valid").is_match("é"));
    }

    #[test]
    fn rejects_bad_patterns() {
        assert!(matches!(Regex::new("("), Err(Error::Compilation(_))));
        // RE2 caps repeat counts at 1000.
        assert!(matches!(Regex::new("a{1001}"), Err(Error::Compilation(_))));
        assert!(Regex::new("a{1000}").is_ok());
    }

    #[test]
    fn re2_dialect() {
        // Perl classes are ASCII.
        let digits = Regex::new(r"^\d+$").expect("valid");
        assert!(digits.is_match("123"));
        assert!(!digits.is_match("١٢٣"));
        // Quoted literals.
        let quoted = Regex::new(r"^\Qa.c\E$").expect("valid");
        assert!(quoted.is_match("a.c"));
        assert!(!quoted.is_match("abc"));
        // A hyphen after a class escape is literal.
        assert!(Regex::new(r"^[\d-z]$").expect("valid").is_match("-"));
    }
}
