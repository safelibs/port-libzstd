use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
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

fn env_tool(name: &str, default: &str) -> OsString {
    env::var_os(name).unwrap_or_else(|| OsString::from(default))
}

fn run_command(command: &mut Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to run {description}: {error}"));
    if !status.success() {
        panic!("{description} failed with status {status}");
    }
}

fn upstream_lib_root(manifest_dir: &Path) -> PathBuf {
    for candidate in [
        manifest_dir.join("original/libzstd-1.5.5+dfsg2/lib"),
        manifest_dir.join("../original/libzstd-1.5.5+dfsg2/lib"),
    ] {
        if candidate.exists() {
            return candidate;
        }
    }

    panic!(
        "could not locate the staged upstream lib sources next to {}",
        manifest_dir.display()
    );
}

fn upstream_phase4_sources(root: &Path) -> Vec<PathBuf> {
    let common = root.join("common");
    let compress = root.join("compress");
    let decompress = root.join("decompress");
    let dict_builder = root.join("dictBuilder");
    vec![
        common.join("debug.c"),
        common.join("entropy_common.c"),
        common.join("error_private.c"),
        common.join("fse_decompress.c"),
        common.join("pool.c"),
        common.join("threading.c"),
        common.join("xxhash.c"),
        common.join("zstd_common.c"),
        compress.join("fse_compress.c"),
        compress.join("hist.c"),
        compress.join("huf_compress.c"),
        compress.join("zstd_compress.c"),
        compress.join("zstd_compress_literals.c"),
        compress.join("zstd_compress_sequences.c"),
        compress.join("zstd_compress_superblock.c"),
        compress.join("zstd_double_fast.c"),
        compress.join("zstd_fast.c"),
        compress.join("zstd_lazy.c"),
        compress.join("zstd_ldm.c"),
        compress.join("zstd_opt.c"),
        compress.join("zstdmt_compress.c"),
        decompress.join("huf_decompress.c"),
        decompress.join("zstd_ddict.c"),
        decompress.join("zstd_decompress.c"),
        decompress.join("zstd_decompress_block.c"),
        dict_builder.join("cover.c"),
        dict_builder.join("divsufsort.c"),
        dict_builder.join("fastcover.c"),
        dict_builder.join("zdict.c"),
    ]
}

fn compile_source(
    compiler: &cc::Tool,
    includes: &[PathBuf],
    source: &Path,
    object: &Path,
) {
    let mut command = Command::new(compiler.path());
    command
        .args(compiler.args())
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(object)
        .arg("-DZSTD_MULTITHREAD")
        .arg("-DZSTD_STATIC_LINKING_ONLY")
        .arg("-DZDICT_STATIC_LINKING_ONLY")
        .arg("-D_POSIX_C_SOURCE=200809L");
    if compiler.is_like_gnu() || compiler.is_like_clang() {
        command.arg("-fvisibility=hidden");
    }
    for include in includes {
        command.arg("-I").arg(include);
    }
    run_command(
        &mut command,
        &format!("compile upstream phase-4 helper {}", source.display()),
    );
}

fn compile_upstream_phase4_helpers(manifest_dir: &Path) {
    let upstream_root = upstream_lib_root(manifest_dir);
    let includes = vec![
        upstream_root.clone(),
        upstream_root.join("common"),
        upstream_root.join("compress"),
        upstream_root.join("decompress"),
        upstream_root.join("dictBuilder"),
    ];
    let sources = upstream_phase4_sources(&upstream_root);

    for source in &sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let build_dir = out_dir.join("upstream-phase4");
    let object_dir = build_dir.join("objects");
    fs::create_dir_all(&object_dir).expect("create upstream helper object dir");

    let mut cc_build = cc::Build::new();
    cc_build
        .warnings(false)
        .include(&upstream_root)
        .include(upstream_root.join("common"))
        .include(upstream_root.join("compress"))
        .include(upstream_root.join("decompress"))
        .include(upstream_root.join("dictBuilder"))
        .define("ZSTD_MULTITHREAD", None)
        .define("ZSTD_STATIC_LINKING_ONLY", None)
        .define("ZDICT_STATIC_LINKING_ONLY", None)
        .define("_POSIX_C_SOURCE", "200809L");
    cc_build.flag_if_supported("-fvisibility=hidden");
    let compiler = cc_build.get_compiler();

    let mut objects = Vec::with_capacity(sources.len());
    for source in &sources {
        let file_name = source
            .file_name()
            .expect("upstream source file name")
            .to_string_lossy()
            .replace('.', "_");
        let object = object_dir.join(format!("{file_name}.o"));
        compile_source(&compiler, &includes, source, &object);
        objects.push(object);
    }

    let nm = env_tool("NM", "nm");
    let objcopy = env_tool("OBJCOPY", "objcopy");
    let ar = env_tool("AR", "ar");

    let mut symbols = BTreeSet::new();
    for object in &objects {
        let output = Command::new(&nm)
            .arg("-g")
            .arg("--defined-only")
            .arg(object)
            .output()
            .unwrap_or_else(|error| panic!("failed to inspect {} with nm: {error}", object.display()));
        if !output.status.success() {
            panic!("nm failed for {}", object.display());
        }
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some(symbol) = line.split_whitespace().last() else {
                continue;
            };
            if symbol.is_empty()
                || symbol.starts_with('.')
                || symbol.starts_with("__gnu_lto")
                || symbol == "_GLOBAL_OFFSET_TABLE_"
            {
                continue;
            }
            symbols.insert(symbol.to_owned());
        }
    }

    let rename_map = build_dir.join("rename.syms");
    let mut rename_lines = String::new();
    for symbol in &symbols {
        rename_lines.push_str(symbol);
        rename_lines.push(' ');
        rename_lines.push_str("libzstd_safe_internal_");
        rename_lines.push_str(symbol);
        rename_lines.push('\n');
    }
    fs::write(&rename_map, rename_lines).expect("write upstream helper symbol map");

    for object in &objects {
        let mut command = Command::new(&objcopy);
        command.arg("--redefine-syms").arg(&rename_map).arg(object);
        run_command(
            &mut command,
            &format!("prefix symbols in {}", object.display()),
        );
    }

    let archive = build_dir.join("libzstd_safe_phase4.a");
    let _ = fs::remove_file(&archive);
    let mut archive_command = Command::new(&ar);
    archive_command.arg("crus").arg(&archive);
    for object in &objects {
        archive_command.arg(object);
    }
    run_command(&mut archive_command, "archive upstream phase-4 helpers");

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=zstd_safe_phase4");
    println!("cargo:rustc-link-lib=pthread");
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
    compile_upstream_phase4_helpers(&manifest_dir);
}
