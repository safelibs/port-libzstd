use std::{
    env,
    path::{Path, PathBuf},
    process::{self},
};

fn feature_enabled(name: &str) -> bool {
    env::var_os(name).is_some()
}

fn emit_cfg(name: &str, enabled: bool) {
    println!("cargo:rustc-check-cfg=cfg({name})");
    if enabled {
        println!("cargo:rustc-cfg={name}");
    }
}

fn upstream_lib_root(manifest_dir: &Path) -> PathBuf {
    for candidate in [
        manifest_dir.join("lib"),
        manifest_dir.join("../original/libzstd-1.5.5+dfsg2/lib"),
    ] {
        if candidate.exists() {
            return candidate;
        }
    }

    panic!(
        "could not locate the upstream lib sources for {}",
        manifest_dir.display()
    );
}
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let requested_threading = feature_enabled("CARGO_FEATURE_THREADING");
    let build_shared_default = feature_enabled("CARGO_FEATURE_BUILD_SHARED_DEFAULT");
    let build_static_default = feature_enabled("CARGO_FEATURE_BUILD_STATIC_DEFAULT");
    let variant_mt = feature_enabled("CARGO_FEATURE_VARIANT_MT");
    let variant_nomt = feature_enabled("CARGO_FEATURE_VARIANT_NOMT");
    let threading = if build_static_default || variant_nomt {
        false
    } else if requested_threading || build_shared_default || variant_mt {
        true
    } else {
        true
    };

    if variant_mt && variant_nomt {
        eprintln!("conflicting libzstd-safe features: `variant-mt` and `variant-nomt`");
        process::exit(1);
    }

    if build_shared_default && build_static_default {
        eprintln!(
            "conflicting libzstd-safe features: `build-shared-default` and `build-static-default`"
        );
        process::exit(1);
    }

    emit_cfg("libzstd_threading", threading);
    emit_cfg("libzstd_build_shared_default", build_shared_default);
    emit_cfg("libzstd_build_static_default", build_static_default);
    emit_cfg("libzstd_variant_mt", variant_mt);
    emit_cfg("libzstd_variant_nomt", variant_nomt);

    let variant_suffix = if variant_mt {
        "-mt"
    } else if variant_nomt {
        "-nomt"
    } else {
        ""
    };

    let default_artifact = if build_shared_default {
        "shared"
    } else if build_static_default {
        "static"
    } else {
        "scaffold"
    };

    println!(
        "cargo:rustc-env=LIBZSTD_THREADING={}",
        if threading { "enabled" } else { "disabled" }
    );
    println!("cargo:rustc-env=LIBZSTD_VARIANT_SUFFIX={variant_suffix}");
    println!("cargo:rustc-env=LIBZSTD_DEFAULT_ARTIFACT={default_artifact}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libzstd.so.1");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let upstream_root = upstream_lib_root(&manifest_dir);
    let legacy_root = upstream_root.join("legacy");
    let common_root = upstream_root.join("common");
    let legacy_files = [
        common_root.join("xxhash.c"),
        legacy_root.join("zstd_v05.c"),
        legacy_root.join("zstd_v06.c"),
        legacy_root.join("zstd_v07.c"),
        manifest_dir.join("src/ffi/legacy_shim.c"),
    ];

    for path in &legacy_files {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        legacy_root.join("zstd_legacy.h").display()
    );

    let mut build = cc::Build::new();
    build
        .warnings(false)
        .include(&legacy_root)
        .include(&common_root)
        .include(&upstream_root)
        .define("ZSTD_LEGACY_SUPPORT", "5");

    for path in &legacy_files {
        build.file(path);
    }

    build.compile("zstd_safe_legacy");
}
