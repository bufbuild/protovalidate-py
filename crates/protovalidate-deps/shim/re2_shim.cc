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
#include "re2_shim.h"

#include <memory>
#include <string>

#include "absl/strings/string_view.h"
#include "cel_shim.h"
#include "internal/re2_options.h"
#include "re2/re2.h"

struct re2_regex {
  explicit re2_regex(absl::string_view pattern)
      : re(pattern, cel::internal::MakeRE2Options()) {}
  RE2 re;
};

extern "C" {

int re2_regex_new(const char* pattern, size_t len, re2_regex** out,
                  char** error) {
  auto regex = std::make_unique<re2_regex>(absl::string_view(pattern, len));
  if (!regex->re.ok()) {
    const std::string& message = regex->re.error();
    *error = cel_string_new(message.data(), message.size());
    return CEL_ERR_COMPILATION;
  }
  *out = regex.release();
  return CEL_OK;
}

void re2_regex_free(re2_regex* regex) { delete regex; }

int re2_regex_matches(const re2_regex* regex, const char* text, size_t len) {
  return RE2::PartialMatch(absl::string_view(text, len), regex->re) ? 1 : 0;
}

}  // extern "C"
