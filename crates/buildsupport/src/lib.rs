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

//! Shared build-script helpers for the native C++ crates.
//!
//! Upstream C++ sources are git submodules under each crate's `third_party/`,
//! pinned to the exact versions bazel resolves. Code that has no upstream file
//! to point at -- protoc output, the ANTLR-generated CEL parser -- is checked
//! in under `gen/`. The per-library lists of C++ sources to compile live in
//! `filelists/`. Both `gen/` and the filelists are produced by
//! `scripts/extract_native_sources.py` from bazel's action graph.
//!
//! Include directories travel between crates through cargo's `links`
//! metadata: a crate with `links = "deps"` publishes `cargo::metadata=include=…`
//! and its dependents read it back as `DEP_DEPS_INCLUDE`. That keeps the
//! crates relocatable, which matters because they are consumed from an sdist
//! unpacked at an arbitrary path.

use std::env;
use std::path::{Path, PathBuf};

/// Separator for multi-path metadata values.
const PATH_SEP: char = ';';

/// The directory of the crate whose build script is running.
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
}

/// Whether the C++ compilation can be skipped for this build. We skip it
/// with clippy or when maturin generates pyi stubs since they don't need the
/// actual native binary built.
///
/// Build scripts return early on this so that a checkout without the
/// `third_party/` submodules can still lint and generate stubs.
pub fn should_skip_cpp() -> bool {
    println!("cargo::rerun-if-env-changed=CLIPPY_ARGS");
    println!("cargo::rerun-if-env-changed=PROTOVALIDATE_SKIP_CPP");
    env::var_os("CLIPPY_ARGS").is_some() || env::var_os("PROTOVALIDATE_SKIP_CPP").is_some()
}

/// A submodule checkout under this crate's `third_party/`.
pub fn third_party_dir(name: &str) -> PathBuf {
    let dir = manifest_dir().join("third_party").join(name);
    assert!(
        dir.join(".git").exists() || dir.read_dir().is_ok_and(|mut d| d.next().is_some()),
        "{} is empty. Run `git submodule update --init --recursive`.",
        dir.display(),
    );
    rerun_if_changed(&dir);
    dir
}

/// The checked-in generated sources for this crate.
pub fn gen_dir() -> PathBuf {
    let dir = manifest_dir().join("gen");
    assert!(dir.is_dir(), "{} does not exist", dir.display());
    rerun_if_changed(&dir);
    dir
}

fn rerun_if_changed(path: &Path) {
    println!("cargo::rerun-if-changed={}", path.display());
}

/// Source files a library compiles, as paths relative to the crate root.
///
/// A `windows:`/`linux:`/`macos:` prefix marks a file bazel adds through a
/// platform select(); it is compiled only when the target OS matches.
fn read_filelist(path: &Path) -> Vec<String> {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");
    println!("cargo::rerun-if-changed={}", path.display());
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

/// Publishes include directories for dependent crates to pick up.
pub fn export_includes(dirs: &[PathBuf]) {
    let joined = dirs
        .iter()
        .map(|d| d.display().to_string())
        .collect::<Vec<_>>()
        .join(&PATH_SEP.to_string());
    println!("cargo::metadata=include={joined}");
}

/// Include directories published by a dependency, keyed by its `links` name.
fn dep_includes(links_name: &str) -> Vec<PathBuf> {
    let key = format!("DEP_{}_INCLUDE", links_name.to_uppercase());
    let value = env::var(&key)
        .unwrap_or_else(|_| panic!("{key} not set; is the crate a dependency with a `links` key?"));
    value.split(PATH_SEP).map(PathBuf::from).collect()
}

/// Routes MSVC C++ compiles through `RUSTC_WRAPPER` when set.
/// cc applies the wrapper only to compilers named via CC/CXX, so
/// the cl.exe it discovers through the registry bypasses that fallback
/// We import the discovered toolchain's environment (PATH/INCLUDE/LIB)
/// and name the compiler explicitly, which sends compiler resolution
/// down the env-var branch where the wrapper is attached.
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
    // SAFETY: build scripts are single-threaded when this runs, before any
    // cc::Build is constructed.
    for (key, value) in tool.env() {
        unsafe { env::set_var(key, value) };
    }
    unsafe {
        // Bare `cl`, resolved via the PATH set for the toolchain.
        env::set_var("CC", "cl");
        env::set_var("CXX", "cl");
    }
}

/// A C++ static library built from vendored sources.
pub struct CxxLib {
    name: String,
    pub build: cc::Build,
    includes: Vec<PathBuf>,
}

impl CxxLib {
    pub fn new(name: &str) -> Self {
        reroute_msvc_through_wrapper();
        let mut build = cc::Build::new();
        build.cpp(true).warnings(false);

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
            // Everything here is an implementation detail of the extension
            // module, which exports only its PyInit symbol.
            build.flag_if_supported("-fvisibility=hidden");
            build.flag_if_supported("-fvisibility-inlines-hidden");
        }

        Self {
            name: name.to_owned(),
            build,
            includes: Vec::new(),
        }
    }

    /// Adds an include directory and records it for [`Self::export`].
    pub fn include(&mut self, dir: impl AsRef<Path>) -> &mut Self {
        let dir = dir.as_ref().to_path_buf();
        self.build.include(&dir);
        self.includes.push(dir);
        self
    }

    /// Adds a dependency's published include directories.
    ///
    /// These are recorded for re-export, so include paths accumulate along the
    /// dependency chain and the crate at the end of it (the extension) gets
    /// every directory it needs from its one direct dependency.
    pub fn include_deps(&mut self, links_names: &[&str]) -> &mut Self {
        for name in links_names {
            for dir in dep_includes(name) {
                self.build.include(&dir);
                self.includes.push(dir);
            }
        }
        self
    }

    pub fn define(&mut self, key: &str, value: Option<&str>) -> &mut Self {
        self.build.define(key, value);
        self
    }

    /// Adds the defines required to compile against the vendored ANTLR4 C++
    /// runtime headers, in the runtime itself and its dependents alike.
    pub fn antlr4_defines(&mut self) -> &mut Self {
        self.define("ANTLR4CPP_STATIC", None)
            .define("ANTLR4CPP_USING_ABSEIL", None)
    }

    /// Adds every C++ source file named in the filelist, whose entries are
    /// paths relative to the crate root.
    pub fn files_from_filelist(&mut self, list: &Path) -> &mut Self {
        let root = manifest_dir();
        for file in read_filelist(list) {
            self.build.file(root.join(file));
        }
        self
    }

    pub fn compile(&mut self) {
        if should_skip_cpp() {
            return;
        }
        self.build.compile(&self.name);
    }

    /// The include directories added via [`Self::include`].
    pub fn includes(&self) -> &[PathBuf] {
        &self.includes
    }

    /// Publishes the include directories added via [`Self::include`].
    pub fn export(&self) {
        export_includes(&self.includes);
    }
}
