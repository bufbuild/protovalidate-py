// Copyright 2026 Buf Technologies, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// A C API over RE2, the regular expression engine cel-cpp's `matches()`
// uses, so that the validator's native `pattern` rules share its syntax.
// Status codes and error strings follow cel_shim.h: an error string is
// allocated with cel_string_new and released with cel_free.

#ifndef PROTOVALIDATE_SHIM_RE2_SHIM_H_
#define PROTOVALIDATE_SHIM_RE2_SHIM_H_

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct re2_regex re2_regex;

// Compiles `pattern` (UTF-8, RE2 syntax) into `*out`. Returns CEL_OK, or
// CEL_ERR_COMPILATION with the engine's message in `*error`.
int re2_regex_new(const char* pattern, size_t len, re2_regex** out,
                  char** error);

void re2_regex_free(re2_regex* regex);

// Whether `text` contains a match of the pattern, as RE2::PartialMatch.
int re2_regex_matches(const re2_regex* regex, const char* text, size_t len);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // PROTOVALIDATE_SHIM_RE2_SHIM_H_
