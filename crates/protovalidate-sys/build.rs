fn main() {
    let vendor = buildsupport::vendor_dir();

    let mut lib = buildsupport::CxxLib::new("protovalidate");
    lib.include(&vendor)
        .include_deps(&["celcpp", "absl", "protobuf", "re2", "antlr4rt"])
        .antlr4_defines()
        .files_from_filelist(&vendor);

    // The C ABI the Rust bindings call through.
    println!("cargo::rerun-if-changed=shim/pv_shim.cc");
    println!("cargo::rerun-if-changed=shim/pv_shim.h");
    lib.include("shim");
    lib.build.file("shim/pv_shim.cc");

    lib.compile();
    lib.export();
}
