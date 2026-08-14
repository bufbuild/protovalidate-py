fn main() {
    if buildsupport::should_skip_cpp() {
        return;
    }
    let pvcc = buildsupport::third_party_dir("protovalidate-cc");
    let generated = buildsupport::gen_dir();

    let mut lib = buildsupport::CxxLib::new("protovalidate");
    lib.include(&pvcc)
        .include(&generated)
        .include_deps(&["deps"])
        .antlr4_defines()
        .files_from_filelist(&buildsupport::manifest_dir().join("filelists/protovalidate.txt"));

    // The C ABI the Rust bindings call through.
    println!("cargo::rerun-if-changed=shim/pv_shim.cc");
    println!("cargo::rerun-if-changed=shim/pv_shim.h");
    lib.include("shim");
    lib.build.file("shim/pv_shim.cc");

    lib.compile();
    lib.export();
}
