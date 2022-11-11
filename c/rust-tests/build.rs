#[cfg(target_env="msvc")]
fn main() {
    println!("cargo:rerun-if-changed=../ckb_smt.h");
    cc::Build::new()
        .file("../../src/ckb_smt.c")
        .static_flag(true)
        .flag("/Ox")
        .flag("/Gw")
        .flag("/Gy")
        .include("../../src")
        .include("..")
        .include("../deps/ckb-c-stdlib")
        .flag("/Wall")
        .define("__SHARED_LIBRARY__", None)
        .define("CKB_STDLIB_NO_SYSCALL_IMPL", None)
        .compile("dl-c-impl");

}

#[cfg(not(target_env="msvc"))]
fn main() {
    println!("cargo:rerun-if-changed=../ckb_smt.h");
    cc::Build::new()
        .file("../../src/ckb_smt.c")
        .static_flag(true)
        .flag("-O3")
        .flag("-fvisibility=hidden")
        .flag("-fdata-sections")
        .flag("-ffunction-sections")
        .include("../../src")
        .include("..")
        .include("../deps/ckb-c-stdlib")
        .flag("-Wall")
        .flag("-Werror")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-nonnull")
        .define("__SHARED_LIBRARY__", None)
        .define("CKB_STDLIB_NO_SYSCALL_IMPL", None)
        .compile("dl-c-impl");

}

