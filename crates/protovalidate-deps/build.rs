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

//! Compiles the CEL runtime behind the shim: cel-cpp and the C++ libraries
//! it builds on, plus the shim itself.
//!
//! Upstream sources are git submodules under `third_party/` with code generated
//! by Bazel (protoc output, the ANTLR-generated CEL parser) checked
//! in under `gen/`. The files each library compiles are listed in
//! `filelists/`.All of it is produced by `scripts/extract_native_sources.py`
//! from bazel's action graph without hand maintenance.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The directory of this crate.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
}

fn rerun_if_changed(path: &Path) {
    println!("cargo::rerun-if-changed={}", path.display());
}

/// Whether to skip the C++ compilation: under clippy, rust-analyzer and
/// docs.rs, none of which links the native code, or when
/// `PROTOVALIDATE_SKIP_CPP` is set.
fn should_skip_cpp() -> bool {
    for var in [
        "CLIPPY_ARGS",
        "DOCS_RS",
        "PROTOVALIDATE_SKIP_CPP",
        "RA_RUSTC_WRAPPER",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
    ] {
        println!("cargo::rerun-if-env-changed={var}");
    }
    let wraps_rust_analyzer = |var: &str| {
        env::var_os(var).is_some_and(|wrapper| wrapper.to_string_lossy().contains("rust-analyzer"))
    };
    env::var_os("CLIPPY_ARGS").is_some()
        || env::var_os("DOCS_RS").is_some()
        || env::var_os("PROTOVALIDATE_SKIP_CPP").is_some()
        || env::var_os("RA_RUSTC_WRAPPER").is_some()
        || wraps_rust_analyzer("RUSTC_WRAPPER")
        || wraps_rust_analyzer("RUSTC_WORKSPACE_WRAPPER")
}

/// A submodule checkout under `third_party/`.
fn third_party_dir(name: &str) -> PathBuf {
    let dir = manifest_dir().join("third_party").join(name);
    let mut entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "{}: {e}. Run `git submodule update --init --recursive`.",
                dir.display()
            )
        })
        .map(|entry| entry.expect("read_dir entry").path())
        .filter(|path| path.file_name() != Some(OsStr::new(".git")))
        .peekable();
    assert!(
        entries.peek().is_some(),
        "{} is empty. Run `git submodule update --init --recursive`.",
        dir.display(),
    );
    // Watch the checkout's contents rather than the directory itself. cargo
    // scans a watched directory recursively, and a submodule keeps its own
    // `.git` directory, which git rewrites on any superproject operation --
    // a pull that changes nothing here still restamps it. Watching the root
    // would rebuild every C++ file each time git touched its bookkeeping.
    for path in entries {
        rerun_if_changed(&path);
    }
    dir
}

/// The checked-in generated sources.
fn gen_dir() -> PathBuf {
    let dir = manifest_dir().join("gen");
    assert!(dir.is_dir(), "{} does not exist", dir.display());
    rerun_if_changed(&dir);
    dir
}

/// Source files a library compiles, as paths relative to the crate root.
///
/// A `windows:`/`linux:`/`macos:` prefix marks a file bazel adds through a
/// platform `select()`; it is compiled only when the target OS matches.
fn read_filelist(path: &Path) -> Vec<String> {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");
    rerun_if_changed(path);
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| match line.split_once(':') {
            Some((os, path)) if ["windows", "linux", "macos"].contains(&os) => {
                (os == target_os).then(|| path.to_owned())
            }
            _ => Some(line.to_owned()),
        })
        .collect()
}

/// Routes MSVC C++ compiles through `RUSTC_WRAPPER` when set.
///
/// cc applies the wrapper only to compilers named via `CC`/`CXX`, not to the
/// cl.exe it finds through the registry, so the discovered toolchain's
/// environment (`PATH`/`INCLUDE`/`LIB`) is imported and the compiler named
/// explicitly.
fn reroute_msvc_through_wrapper() {
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
        || env::var_os("RUSTC_WRAPPER").is_none()
        || env::var_os("CC").is_some()
        || env::var_os("CXX").is_some()
    {
        return;
    }
    let target = env::var("TARGET").expect("TARGET");
    let Some(tool) = cc::windows_registry::find_tool(&target, "cl.exe") else {
        return;
    };
    for (key, value) in tool.env() {
        // SAFETY: build scripts are single-threaded when this runs, before any
        // cc::Build is constructed.
        unsafe { env::set_var(key, value) };
    }
    // SAFETY: as above.
    unsafe {
        // Bare `cl`, resolved via the PATH set for the toolchain.
        env::set_var("CC", "cl");
        env::set_var("CXX", "cl");
    }
}

/// Every C++ source under `dir`, recursively, in a stable order.
fn cxx_sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("read_dir entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "cc") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// One static library built from the C++ sources.
struct CxxLib {
    name: String,
    build: cc::Build,
}

impl CxxLib {
    /// A library over the files `filelists/<name>.txt` records.
    fn new(name: &str, includes: &[&Path]) -> Self {
        let root = manifest_dir();
        let files = read_filelist(&root.join(format!("filelists/{name}.txt")))
            .into_iter()
            .map(|file| root.join(file))
            .collect();
        Self::with_files(name, includes, files)
    }

    fn with_files(name: &str, includes: &[&Path], files: Vec<PathBuf>) -> Self {
        reroute_msvc_through_wrapper();
        let mut build = cc::Build::new();
        // Debug info bloats target directory for no real benefit.
        build.cpp(true).warnings(false).debug(false);

        if build.get_compiler().is_like_msvc() {
            build.std("c++20");
            // msvc does not support cel-cpp's constinit so we clear it
            build.define("constinit", "");
            build.define("_ALLOW_KEYWORD_MACROS", None);
            // Standard unwind semantics, which cc does not add on its own.
            build.flag("/EHsc").flag("/utf-8").flag("/bigobj");
            build.define("NOMINMAX", None);
            build.define("WIN32_LEAN_AND_MEAN", None);
        } else {
            build.std("c++17");
            build.flag_if_supported("-fsized-deallocation");
            build.flag_if_supported("-faligned-allocation");
            // Everything here is an implementation detail of this crate;
            // no C++ symbol is part of its public surface.
            build.flag_if_supported("-fvisibility=hidden");
            build.flag_if_supported("-fvisibility-inlines-hidden");
        }
        for dir in includes {
            build.include(dir);
        }

        for file in files {
            build.file(file);
        }
        Self {
            name: name.to_owned(),
            build,
        }
    }

    /// Adds the defines required to compile against the ANTLR4 C++ runtime
    /// headers, in the runtime itself and its dependents alike.
    fn antlr4_defines(&mut self) -> &mut Self {
        self.build.define("ANTLR4CPP_STATIC", None);
        self.build.define("ANTLR4CPP_USING_ABSEIL", None);
        self
    }

    fn compile(&mut self) {
        self.build.compile(&self.name);
    }
}

fn main() {
    if should_skip_cpp() {
        return;
    }
    let absl = third_party_dir("abseil-cpp");
    let antlr4 = third_party_dir("antlr4").join("runtime/Cpp/runtime/src");
    let re2 = third_party_dir("re2");
    let protobuf_root = third_party_dir("protobuf");
    let celcpp = third_party_dir("cel-cpp");
    let generated = gen_dir();

    let protobuf = protobuf_root.join("src");
    let utf8 = protobuf_root.join("third_party/utf8_range");
    let shim = manifest_dir().join("shim");

    // gen/ is listed after the upstream include roots so that where protobuf
    // ships a checked-in copy of a generated header, the checked-in one wins.
    let mut absl_lib = CxxLib::new("absl", &[&absl]);

    let mut antlr4_lib = CxxLib::new("antlr4", &[&antlr4, &absl]);
    antlr4_lib.antlr4_defines();

    let mut re2_lib = CxxLib::new("re2", &[&re2, &absl]);

    let mut protobuf_lib = CxxLib::new("protobuf", &[&protobuf, &utf8, &generated, &absl]);

    let mut celcpp_lib = CxxLib::new(
        "celcpp",
        &[&celcpp, &generated, &absl, &protobuf, &utf8, &re2, &antlr4],
    );
    celcpp_lib.antlr4_defines();

    rerun_if_changed(&shim);
    let mut shim_lib = CxxLib::with_files(
        "celshim",
        &[
            &shim, &generated, &celcpp, &absl, &protobuf, &utf8, &re2, &antlr4,
        ],
        cxx_sources(&shim),
    );
    shim_lib.antlr4_defines();

    // Static archives must precede the archives they reference on the link
    // line, so compile (and therefore emit link directives) most-dependent
    // first.
    shim_lib.compile();
    celcpp_lib.compile();
    protobuf_lib.compile();
    re2_lib.compile();
    antlr4_lib.compile();
    absl_lib.compile();

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo::rustc-link-lib=framework=CoreFoundation");
    }
}
