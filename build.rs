fn main() {
    cc::Build::new()
        .file("src/nvma/nvma_lib.c")
        .include("nvma")
        .warnings(false)
        .compile("nvma");
}
