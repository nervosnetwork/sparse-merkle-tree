fn main() {
    println!("cargo:rerun-if-changed=c/ckb_smt.h");

    cc::Build::new()
        .file("src/c/ckb_smt.c")
        .static_flag(true)
        .flag("-O3")
        .flag("-fvisibility=hidden")
        .flag("-fdata-sections")
        .flag("-ffunction-sections")
        .include("src/c")
        .include("c/")
        .include("../c/deps/ckb-c-stdlib")
        .flag("-Wall")
        .flag("-Werror")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-nonnull")
        .define("__SHARED_LIBRARY__", None)
        .define("CKB_STDLIB_NO_SYSCALL_IMPL", None)
        .compile("dl-c-impl");
}
