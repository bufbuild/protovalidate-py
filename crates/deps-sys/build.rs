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

use std::path::{Path, PathBuf};

use buildsupport::CxxLib;

fn lib(name: &str, includes: &[&Path]) -> CxxLib {
    let mut lib = CxxLib::new(name);
    for dir in includes {
        lib.include(dir);
    }
    lib.files_from_filelist(&buildsupport::manifest_dir().join(format!("filelists/{name}.txt")));
    lib
}

fn main() {
    if buildsupport::should_skip_cpp() {
        return;
    }
    let absl_dir = buildsupport::third_party_dir("abseil-cpp");
    let antlr4_dir = buildsupport::third_party_dir("antlr4").join("runtime/Cpp/runtime/src");
    let re2_dir = buildsupport::third_party_dir("re2");
    let protobuf = buildsupport::third_party_dir("protobuf");
    let celcpp_dir = buildsupport::third_party_dir("cel-cpp");
    let generated = buildsupport::gen_dir();

    let protobuf_dir = protobuf.join("src");
    let utf8_dir = protobuf.join("third_party/utf8_range");

    // protoc output and the ANTLR-generated CEL parser live in gen/. They are
    // listed after the upstream include roots so that where protobuf ships a
    // checked-in copy of a generated header, the checked-in one still wins --
    // matching how these sources were compiled before the split.
    let mut absl = lib("absl", &[&absl_dir]);
    absl.define("NOMINMAX", None);

    let mut antlr4 = lib("antlr4", &[&antlr4_dir, &absl_dir]);
    antlr4.antlr4_defines();

    let mut re2 = lib("re2", &[&re2_dir, &absl_dir]);

    let mut protobuf_lib = lib("protobuf", &[&protobuf_dir, &utf8_dir, &generated, &absl_dir]);

    let mut celcpp = lib("celcpp", &[
        &celcpp_dir,
        &generated,
        &absl_dir,
        &protobuf_dir,
        &utf8_dir,
        &re2_dir,
        &antlr4_dir,
    ]);
    celcpp.antlr4_defines();

    // Static archives must precede the archives they reference on the link
    // line, so compile (and therefore emit link directives) most-dependent
    // first.
    celcpp.compile();
    protobuf_lib.compile();
    re2.compile();
    antlr4.compile();
    absl.compile();

    // Everything a dependent needs to compile against these libraries.
    let includes: Vec<PathBuf> = [
        celcpp_dir,
        generated,
        absl_dir,
        protobuf_dir,
        utf8_dir,
        re2_dir,
        antlr4_dir,
    ]
    .into();
    buildsupport::export_includes(&includes);

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo::rustc-link-lib=framework=CoreFoundation");
    }
}
