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

//! The C++ libraries protovalidate-cc builds on: Abseil, protobuf, RE2, the
//! ANTLR4 C++ runtime, and cel-cpp.
//!
//! This crate has no Rust API. Its build script compiles each library from the
//! submodules under `third_party/` and publishes their include directories for
//! `protovalidate-sys` to compile against.
