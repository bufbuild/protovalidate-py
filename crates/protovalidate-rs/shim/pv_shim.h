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

// C ABI over protovalidate-cc. Nothing but bytes and primitives crosses this
// boundary: descriptors arrive as serialized google.protobuf.FileDescriptorProto,
// payloads as serialized messages, results as a serialized
// buf.validate.Violations.

#ifndef PROTOVALIDATE_EXT_PV_SHIM_H_
#define PROTOVALIDATE_EXT_PV_SHIM_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct pv_engine pv_engine;

// Status codes returned by the fallible entry points. The Python layer maps
// these onto exception types, and the conformance harness buckets results by
// exception type, so the compilation/runtime distinction is observable
// behaviour rather than an internal detail.
enum {
  PV_OK = 0,
  PV_ERR_COMPILATION = 1,  // rules could not be compiled
  PV_ERR_RUNTIME = 2,      // a rule failed while being evaluated
  PV_ERR_ARGUMENT = 3,     // bad descriptor / unknown type / unparsable payload
  PV_ERR_UNEXPECTED = 4,   // a status protovalidate-cc does not otherwise use
};

// Creates an engine: a descriptor pool layered over the descriptors linked into
// this extension (well-known types and buf.validate), a dynamic message
// factory, and a protovalidate ValidatorFactory bound to both.
//
// On failure returns NULL and, if `error` is non-NULL, stores a malloc'd
// message in *error which the caller must release with pv_free.
pv_engine* pv_engine_new(char** error);

void pv_engine_free(pv_engine* engine);

// Adds one serialized FileDescriptorProto to the engine's pool. Adding a file
// whose name is already known is a no-op success, so callers may re-send
// descriptors without tracking what the engine has seen.
//
// Files may reference imports that have not been added yet; the pool is
// database-backed and resolves lazily when a type is first looked up.
int pv_engine_add_file(pv_engine* engine, const uint8_t* file_descriptor_proto,
                       size_t len, char** error);

// Validates `payload`, a serialized message of the fully-qualified type named
// by `type_name` and `type_name_len`. The name need not be NUL-terminated:
// the pool lookup takes a string view, so callers pass their own string
// storage directly.
//
// On PV_OK, *out receives a malloc'd buffer of length *out_len holding a
// serialized buf.validate.Violations — empty when the message is valid. The
// caller must release it with pv_free. On failure returns one of the
// PV_ERR_* codes and stores a malloc'd message in *error.
int pv_engine_validate(pv_engine* engine, const char* type_name,
                       size_t type_name_len, const uint8_t* payload,
                       size_t payload_len, int fail_fast, uint8_t** out,
                       size_t* out_len, char** error);

// Releases a buffer returned by pv_engine_validate or an error string.
void pv_free(void* ptr);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // PROTOVALIDATE_EXT_PV_SHIM_H_
