#[cfg(feature = "smtc")]
use std::env;

fn main() {
    #[cfg(feature = "smtc")]
    {
        println!("cargo:rerun-if-changed=c/ckb_smt.h");
        println!("cargo:rerun-if-changed=src/ckb_smt.c");

        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

        let mut build = cc::Build::new();

        build
            .file("src/ckb_smt.c")
            .include("src/")
            .include("c/")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-nonnull");
        if target_arch == "riscv64" {
            build.include("c/deps/ckb-c-stdlib/libc");
            setup_compiler_riscv(&mut build);
        } else {
            setup_compiler_native(&mut build);
        }
        build.compile("smt-c-impl");
    }
}

#[cfg(feature = "smtc")]
fn setup_compiler_riscv(build: &mut cc::Build) {
    build
        .static_flag(true)
        .flag("-fno-builtin-printf")
        .flag("-fno-builtin-memcmp")
        .flag("-nostdinc")
        .flag("-nostdlib")
        .flag("-fvisibility=hidden")
        .flag("-fdata-sections")
        .flag("-ffunction-sections")
        .flag("-Wall")
        .flag("-Werror")
        .define("__SHARED_LIBRARY__", None);

    let clang = match std::env::var_os("CLANG") {
        Some(val) => val,
        None => "clang-19".into(),
    };

    if cfg!(feature = "build-with-clang") {
        build.compiler(clang);
    }

    let compiler = build.get_compiler();
    if compiler.is_like_clang() {
        build
            .no_default_flags(true)
            .flag("--target=riscv64")
            .flag("-march=rv64imc_zba_zbb_zbc_zbs");

        if env::var("DEBUG").map(|v| v != "false").unwrap_or(false) {
            build.flag("-g").flag("-fno-omit-frame-pointer");
        }

        let opt_level = env::var("OPT_LEVEL").expect("fetching OPT_LEVEL");
        if opt_level == "z" {
            build.flag("-Os");
        } else {
            build.flag(format!("-O{}", opt_level));
        }
    } else if compiler.is_like_gnu() {
        build
            .flag("-nostartfiles")
            .flag("-Wno-dangling-pointer")
            .flag("-Wno-nonnull-compare");
    }
}

#[cfg(feature = "smtc")]
fn setup_compiler_native(build: &mut cc::Build) {
    build
        .static_flag(true)
        .flag("-O3")
        .flag("-fvisibility=hidden")
        .flag("-fdata-sections")
        .flag("-ffunction-sections")
        .flag("-Wall")
        .flag("-Werror")
        .define("CKB_STDLIB_NO_SYSCALL_IMPL", None);
}
