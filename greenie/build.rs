fn main() {
    cc::Build::new()
        .file("src/clib_x64.s")
        .compile("libgreenie_clib_x64")
}
