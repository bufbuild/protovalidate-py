fn main() {
    let vendor = buildsupport::vendor_dir();
    let mut lib = buildsupport::CxxLib::new("antlr4rt");
    lib.include(vendor.join("runtime/src"))
        .include_deps(&["absl"])
        .antlr4_defines()
        .files_from_filelist(&vendor);
    lib.compile();
    lib.export();
}
