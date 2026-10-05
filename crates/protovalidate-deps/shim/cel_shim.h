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

// The unsafe C ABI over cel-cpp that the `protovalidate-deps` crate calls.
// As much as possible, logic including validation is kept to the Rust layer
// instead of here.

#ifndef PROTOVALIDATE_SHIM_CEL_SHIM_H_
#define PROTOVALIDATE_SHIM_CEL_SHIM_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// A descriptor pool, message factory, and CEL expression builder.
// cel_engine_add_file, cel_engine_register and cel_program_new mutate it.
// cel_frame_new only reads the pool and the message factory, both safe for
// concurrent readers.
typedef struct cel_engine cel_engine;

// A set of compiled CEL expressions sharing one `rules` message. Immutable
// once built, but bound to the engine's function registry: the engine must
// outlive it.
typedef struct cel_program cel_program;

// A parsed message, owning the arena it lives in. Its type comes from the
// engine's message factory, so the engine must outlive it too.
typedef struct cel_frame cel_frame;

// Status codes returned by the fallible entry points. Every failure stores a
// malloc'd message in *error, which the caller releases with cel_free.
enum {
  CEL_OK = 0,
  CEL_ERR_COMPILATION = 1,  // an expression could not be compiled
  CEL_ERR_RUNTIME = 2,      // an expression failed while being evaluated
  CEL_ERR_ARGUMENT = 3,     // bad descriptor / unknown type / unparsable payload
};

// The kind of a cel_value. CEL_VALUE_OTHER is anything without a cel_value
// form -- a message, a map, null -- where it can be handed over at all.
enum {
  CEL_VALUE_OTHER = 0,
  CEL_VALUE_BOOL = 1,
  CEL_VALUE_INT = 2,     // int32/int64/sint*/sfixed*/enum
  CEL_VALUE_UINT = 3,    // uint32/uint64/fixed*
  CEL_VALUE_DOUBLE = 4,  // float/double
  CEL_VALUE_STRING = 5,
  CEL_VALUE_BYTES = 6,
  CEL_VALUE_LIST = 7,  // only as an argument of a registered function
};

typedef struct cel_list cel_list;

// A value. `data`/`len` are only read for strings and bytes, `list` only for
// lists. Unless a function says otherwise they are borrowed for the call.
typedef struct cel_value {
  int32_t kind;
  int32_t bool_value;
  int64_t int_value;
  uint64_t uint_value;
  double double_value;
  const uint8_t* data;
  size_t len;
  const cel_list* list;
} cel_value;

// One expression to compile. `rule_field_number` names the field of the
// rules message that the `rule` variable is bound to while this expression
// runs, or 0 for none.
typedef struct cel_rule {
  const char* expression;
  size_t expression_len;
  int32_t rule_field_number;
} cel_rule;

// What the `this` variable is bound to during evaluation.
enum {
  CEL_THIS_SCALAR = 0,   // `scalar`
  CEL_THIS_MESSAGE = 1,  // the frame's message
  CEL_THIS_FIELD = 2,    // field `field_number` of the frame's message: a list,
                        // a map, or a singular value, by the field's descriptor
};

// Creates an engine over a descriptor pool layered on the descriptors linked
// into this library (the well-known types).
cel_engine* cel_engine_new(char** error);

void cel_engine_free(cel_engine* engine);

// A predicate implemented by the caller. `args` are the call's arguments, of
// the kinds the function was registered with. Returns CEL_OK with the result
// in *out, or another code with a message from cel_string_new in *error,
// which the expression sees as an error value.
typedef int (*cel_native_fn)(void* ctx, const cel_value* args, size_t len,
                             int* out, char** error);

// Registers `fn` as CEL function `name`, callable on arguments of the given
// kinds (CEL_VALUE_*, one per argument); `receiver_style` makes the first
// argument the receiver, as in `this.isEmail()`. The same name may be
// registered with several kind lists, which CEL resolves by argument type.
// Register before compiling anything.
int cel_engine_register(cel_engine* engine, const char* name, size_t name_len,
                        int receiver_style, const int32_t* arg_kinds,
                        size_t arity, cel_native_fn fn, void* ctx,
                        char** error);

// A list passed to a registered function, valid for that call.
size_t cel_list_len(const cel_list* list);

// Reads element `index` into *out; one without a cel_value form reads as
// CEL_VALUE_OTHER.
void cel_list_get(const cel_list* list, size_t index, cel_value* out);

// Allocates a string the shim releases: for a registered function's error.
char* cel_string_new(const char* data, size_t len);

// Adds one serialized FileDescriptorProto to the engine's pool. Adding a file
// the pool already has -- linked in, or added before -- is a no-op success.
// A file's imports must be added before the file itself.
int cel_engine_add_file(cel_engine* engine, const uint8_t* file_descriptor_proto,
                       size_t len, char** error);

// Compiles expressions that share one `rules` message: a serialized message
// of the type named by `rules_type_name`, or none when `rules_type_name_len`
// is 0, in which case `rules` is bound to null.
//
// On CEL_OK stores the program in *out. Returns CEL_ERR_COMPILATION for an
// expression that does not compile and CEL_ERR_ARGUMENT for an unknown rules
// type, unparsable rules, or a rule field that does not exist.
int cel_program_new(cel_engine* engine, const char* rules_type_name,
                   size_t rules_type_name_len, const uint8_t* rules,
                   size_t rules_len, const cel_rule* exprs, size_t exprs_len,
                   cel_program** out, char** error);

void cel_program_free(cel_program* program);

// Parses `payload` as the message type named by `type_name`. On CEL_OK stores
// the frame in *out; returns CEL_ERR_ARGUMENT for an unknown type or a payload
// that does not parse.
int cel_frame_new(cel_engine* engine, const char* type_name,
                 size_t type_name_len, const uint8_t* payload,
                 size_t payload_len, cel_frame** out, char** error);

void cel_frame_free(cel_frame* frame);

// Evaluates expression `index` of `program` against `this`, described by
// `this_kind` and, depending on it, `scalar`, `frame`, and `field_number`.
//
// On CEL_OK stores the result in *out: CEL_VALUE_BOOL, CEL_VALUE_STRING with
// `data` malloc'd for the caller to release with cel_free, or CEL_VALUE_OTHER
// for anything else. Returns CEL_ERR_RUNTIME when the expression fails to
// evaluate or produces an error value, and CEL_ERR_ARGUMENT for a `this`
// field that does not exist.
int cel_program_eval(const cel_program* program, size_t index, int this_kind,
                    const cel_value* scalar, const cel_frame* frame,
                    int32_t field_number, cel_value* out, char** error);

// Releases a string or buffer the shim allocated.
void cel_free(void* ptr);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // PROTOVALIDATE_SHIM_CEL_SHIM_H_
