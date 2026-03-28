use std::{env, process};

fn feature_enabled(name: &str) -> bool {
    env::var_os(name).is_some()
}

fn emit_cfg(name: &str, enabled: bool) {
    println!("cargo:rustc-check-cfg=cfg({name})");
    if enabled {
        println!("cargo:rustc-cfg={name}");
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let threading = feature_enabled("CARGO_FEATURE_THREADING");
    let build_shared_default = feature_enabled("CARGO_FEATURE_BUILD_SHARED_DEFAULT");
    let build_static_default = feature_enabled("CARGO_FEATURE_BUILD_STATIC_DEFAULT");
    let variant_mt = feature_enabled("CARGO_FEATURE_VARIANT_MT");
    let variant_nomt = feature_enabled("CARGO_FEATURE_VARIANT_NOMT");

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
}
