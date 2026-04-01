#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
UPSTREAM_SO="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5"
EXPORTS_OUT="$SAFE_ROOT/abi/original.exports.txt"
SONAME_OUT="$SAFE_ROOT/abi/original.soname.txt"
EXPORT_MAP_OUT="$SAFE_ROOT/abi/export_map.toml"

MODE=write
if [[ "${1:-}" == "--check" ]]; then
  MODE=check
elif [[ $# -gt 0 ]]; then
  echo "usage: $0 [--check]" >&2
  exit 2
fi

python3 - "$MODE" "$UPSTREAM_SO" "$EXPORTS_OUT" "$SONAME_OUT" "$EXPORT_MAP_OUT" "$REPO_ROOT" <<'PY'
from __future__ import annotations

import pathlib
import re
import subprocess
import sys
import tomllib


def strip_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    text = re.sub(r"//.*", "", text)
    return text


def run(*args: str) -> str:
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def classify_phase(name: str) -> int:
    phase1 = {
        "ZSTD_versionNumber",
        "ZSTD_versionString",
        "ZSTD_isError",
        "ZSTD_getErrorName",
        "ZSTD_getErrorCode",
        "ZSTD_getErrorString",
        "ZSTD_getFrameContentSize",
        "ZSTD_getDecompressedSize",
        "ZSTD_findFrameCompressedSize",
        "ZSTD_findDecompressedSize",
        "ZSTD_decompressBound",
        "ZSTD_frameHeaderSize",
        "ZSTD_getFrameHeader",
        "ZSTD_getFrameHeader_advanced",
        "ZSTD_isFrame",
        "ZSTD_isSkippableFrame",
        "ZSTD_readSkippableFrame",
        "ZSTD_decompressionMargin",
        "ZSTD_decompress",
        "ZSTD_decompressDCtx",
        "ZSTD_createDCtx",
        "ZSTD_freeDCtx",
        "ZSTD_copyDCtx",
        "ZSTD_DCtx_reset",
        "ZSTD_createDStream",
        "ZSTD_freeDStream",
        "ZSTD_initDStream",
        "ZSTD_resetDStream",
        "ZSTD_DCtx_setParameter",
        "ZSTD_DCtx_getParameter",
        "ZSTD_DCtx_setFormat",
        "ZSTD_DCtx_setMaxWindowSize",
        "ZSTD_createDDict",
        "ZSTD_freeDDict",
        "ZSTD_DCtx_loadDictionary",
        "ZSTD_DCtx_refDDict",
        "ZSTD_DCtx_refPrefix",
        "ZSTD_decompress_usingDict",
        "ZSTD_decompress_usingDDict",
        "ZSTD_initDStream_usingDict",
        "ZSTD_initDStream_usingDDict",
        "ZSTD_getDictID_fromDict",
        "ZSTD_getDictID_fromDDict",
        "ZSTD_getDictID_fromFrame",
        "ZSTD_decompressStream",
        "ZSTD_DStreamInSize",
        "ZSTD_DStreamOutSize",
        "ZSTD_nextSrcSizeToDecompress",
        "ZSTD_nextInputType",
        "ZSTD_decodingBufferSize_min",
        "ZSTD_sizeof_DCtx",
        "ZSTD_sizeof_DStream",
        "ZSTD_sizeof_DDict",
        "ZSTD_decompressBegin",
        "ZSTD_decompressBegin_usingDict",
        "ZSTD_decompressBegin_usingDDict",
        "ZSTD_decompressContinue",
        "ZSTD_decompressBlock",
    }
    phase2 = {
        "ZSTD_compressBound",
        "ZSTD_compress",
        "ZSTD_compressCCtx",
        "ZSTD_compress2",
        "ZSTD_createCCtx",
        "ZSTD_freeCCtx",
        "ZSTD_copyCCtx",
        "ZSTD_CCtx_reset",
        "ZSTD_CCtx_setParameter",
        "ZSTD_CCtx_setPledgedSrcSize",
        "ZSTD_CCtx_getParameter",
        "ZSTD_cParam_getBounds",
        "ZSTD_dParam_getBounds",
        "ZSTD_getCParams",
        "ZSTD_getParams",
        "ZSTD_checkCParams",
        "ZSTD_adjustCParams",
        "ZSTD_maxCLevel",
        "ZSTD_minCLevel",
        "ZSTD_defaultCLevel",
        "ZSTD_getDictID_fromCDict",
        "ZSTD_createCStream",
        "ZSTD_freeCStream",
        "ZSTD_initCStream",
        "ZSTD_initCStream_srcSize",
        "ZSTD_initCStream_usingDict",
        "ZSTD_initCStream_usingCDict",
        "ZSTD_initCStream_advanced",
        "ZSTD_initCStream_usingCDict_advanced",
        "ZSTD_resetCStream",
        "ZSTD_compressStream",
        "ZSTD_compressStream2",
        "ZSTD_flushStream",
        "ZSTD_endStream",
        "ZSTD_CStreamInSize",
        "ZSTD_CStreamOutSize",
        "ZSTD_sizeof_CCtx",
        "ZSTD_sizeof_CStream",
        "ZSTD_sizeof_CDict",
        "ZSTD_compressBegin",
        "ZSTD_compressBegin_usingDict",
        "ZSTD_compressBegin_usingCDict",
        "ZSTD_compressBegin_advanced",
        "ZSTD_compressBegin_usingCDict_advanced",
        "ZSTD_compressContinue",
        "ZSTD_compressEnd",
        "ZSTD_getBlockSize",
        "ZSTD_compressBlock",
        "ZSTD_createCDict",
        "ZSTD_freeCDict",
        "ZSTD_CCtx_loadDictionary",
        "ZSTD_CCtx_refCDict",
        "ZSTD_CCtx_refPrefix",
        "ZSTD_compress_usingDict",
        "ZSTD_compress_usingCDict",
    }
    if name in phase1:
        return 1
    if name in phase2:
        return 2
    return 3


def owner_module(name: str) -> str:
    if name.startswith("ZDICT_"):
        if "fastCover" in name:
            return "crate::dict_builder::fastcover"
        if "cover" in name:
            return "crate::dict_builder::cover"
        return "crate::dict_builder::zdict"
    if name in {"ZSTD_versionNumber", "ZSTD_versionString"}:
        return "crate::common::version"
    if name in {"ZSTD_isError", "ZSTD_getErrorName", "ZSTD_getErrorCode", "ZSTD_getErrorString"}:
        return "crate::common::error"
    if name in {"ZSTD_readSkippableFrame", "ZSTD_writeSkippableFrame"}:
        return "crate::common::skippable"
    if name in {
        "ZSTD_getFrameContentSize",
        "ZSTD_getDecompressedSize",
        "ZSTD_findFrameCompressedSize",
        "ZSTD_findDecompressedSize",
        "ZSTD_decompressBound",
        "ZSTD_frameHeaderSize",
        "ZSTD_getFrameHeader",
        "ZSTD_getFrameHeader_advanced",
        "ZSTD_isFrame",
        "ZSTD_isSkippableFrame",
        "ZSTD_decompressionMargin",
        "ZSTD_getDictID_fromFrame",
    }:
        return "crate::common::frame"
    if name in {"ZSTD_createThreadPool", "ZSTD_freeThreadPool", "ZSTD_CCtx_refThreadPool"}:
        return "crate::threading::pool"
    if name in {"ZSTD_getFrameProgression", "ZSTD_toFlushNow"}:
        return "crate::threading::zstdmt"
    if name in {"ZSTD_generateSequences", "ZSTD_compressSequences", "ZSTD_mergeBlockDelimiters", "ZSTD_registerSequenceProducer", "ZSTD_sequenceBound"}:
        return "crate::compress::sequence_api"
    if name in {"ZSTD_createCCtxParams", "ZSTD_freeCCtxParams"} or name.startswith("ZSTD_CCtxParams_"):
        return "crate::compress::cctx_params"
    if name.startswith("ZSTD_DCtx_") or name.startswith("ZSTD_createD") or name.startswith("ZSTD_freeD") or name.startswith("ZSTD_initD") or name.startswith("ZSTD_resetD"):
        if "Dict" in name:
            return "crate::decompress::ddict"
        if "Stream" in name:
            return "crate::decompress::dstream"
        return "crate::decompress::dctx"
    if name.startswith("ZSTD_decompress"):
        if "Block" in name:
            return "crate::decompress::block"
        if "Stream" in name:
            return "crate::decompress::dstream"
        return "crate::decompress::dctx"
    if name in {"ZSTD_nextSrcSizeToDecompress", "ZSTD_nextInputType", "ZSTD_DStreamInSize", "ZSTD_DStreamOutSize", "ZSTD_decodingBufferSize_min"}:
        return "crate::decompress::dstream"
    if name in {
        "ZSTD_createDDict",
        "ZSTD_createDDict_advanced",
        "ZSTD_createDDict_byReference",
        "ZSTD_freeDDict",
        "ZSTD_getDictID_fromDDict",
        "ZSTD_getDictID_fromDict",
        "ZSTD_initDStream_usingDDict",
        "ZSTD_sizeof_DDict",
    }:
        return "crate::decompress::ddict"
    if name in {"ZSTD_copyDCtx"}:
        return "crate::decompress::dctx"
    if name in {"ZSTD_sizeof_DCtx", "ZSTD_estimateDCtxSize"}:
        return "crate::decompress::dctx"
    if name in {"ZSTD_sizeof_DStream", "ZSTD_estimateDStreamSize", "ZSTD_estimateDStreamSize_fromFrame"}:
        return "crate::decompress::dstream"
    if name in {"ZSTD_copyCCtx"}:
        return "crate::compress::cctx"
    if name.startswith("ZSTD_CCtx_") or name.startswith("ZSTD_createC") or name.startswith("ZSTD_freeC"):
        if "Dict" in name:
            return "crate::compress::cdict"
        if "Stream" in name:
            return "crate::compress::cstream"
        if "Params" in name:
            return "crate::compress::cctx_params"
        return "crate::compress::cctx"
    if name in {"ZSTD_flushStream", "ZSTD_endStream"}:
        return "crate::compress::cstream"
    if name.startswith("ZSTD_compress") or name.startswith("ZSTD_initCStream") or name.startswith("ZSTD_resetCStream"):
        if "Sequences" in name:
            return "crate::compress::sequence_api"
        if "Block" in name:
            return "crate::compress::block"
        if "Dict" in name and "usingDict" not in name:
            return "crate::compress::cdict"
        if "Stream" in name or name in {"ZSTD_flushStream", "ZSTD_endStream"}:
            return "crate::compress::cstream"
        return "crate::compress::cctx"
    if name in {
        "ZSTD_getDictID_fromCDict",
        "ZSTD_createCDict",
        "ZSTD_createCDict_advanced",
        "ZSTD_createCDict_advanced2",
        "ZSTD_createCDict_byReference",
        "ZSTD_freeCDict",
        "ZSTD_sizeof_CDict",
        "ZSTD_estimateCDictSize",
        "ZSTD_estimateCDictSize_advanced",
    }:
        return "crate::compress::cdict"
    if name in {"ZSTD_CStreamInSize", "ZSTD_CStreamOutSize", "ZSTD_sizeof_CStream", "ZSTD_estimateCStreamSize", "ZSTD_estimateCStreamSize_usingCParams", "ZSTD_estimateCStreamSize_usingCCtxParams"}:
        return "crate::compress::cstream"
    if name in {"ZSTD_sizeof_CCtx", "ZSTD_estimateCCtxSize", "ZSTD_estimateCCtxSize_usingCParams", "ZSTD_estimateCCtxSize_usingCCtxParams", "ZSTD_compressBound"}:
        return "crate::compress::cctx"
    if name in {"ZSTD_cParam_getBounds", "ZSTD_dParam_getBounds", "ZSTD_getCParams", "ZSTD_getParams", "ZSTD_checkCParams", "ZSTD_adjustCParams", "ZSTD_maxCLevel", "ZSTD_minCLevel", "ZSTD_defaultCLevel", "ZSTD_CCtx_setCParams", "ZSTD_CCtx_setFParams", "ZSTD_CCtx_setParams"}:
        return "crate::compress::params"
    if name.startswith("ZSTD_estimate") or name.startswith("ZSTD_initStatic"):
        return "crate::compress::static_ctx"
    if name in {"ZSTD_getBlockSize", "ZSTD_compressBlock", "ZSTD_insertBlock"}:
        if "decompress" in name.lower():
            return "crate::decompress::block"
        return "crate::compress::block"
    return "crate::ffi::advanced"


mode = sys.argv[1]
upstream_so = pathlib.Path(sys.argv[2])
exports_out = pathlib.Path(sys.argv[3])
soname_out = pathlib.Path(sys.argv[4])
export_map_out = pathlib.Path(sys.argv[5])
repo_root = pathlib.Path(sys.argv[6])

if mode == "check":
    if not exports_out.exists():
        raise SystemExit(f"missing checked-in exports baseline: {exports_out}")
    if not soname_out.exists():
        raise SystemExit(f"missing checked-in SONAME baseline: {soname_out}")
    if not export_map_out.exists():
        raise SystemExit(f"missing checked-in export map: {export_map_out}")

    export_names = []
    for line in exports_out.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        export_names.append(line.split("\t", 1)[0])
    if not export_names:
        raise SystemExit("checked-in exports baseline is empty")

    soname = soname_out.read_text(encoding="utf-8").strip()
    if not soname:
        raise SystemExit("checked-in SONAME baseline is empty")

    export_map = tomllib.loads(export_map_out.read_text(encoding="utf-8"))
    symbols = export_map.get("symbol", [])
    if [entry["name"] for entry in symbols] != export_names:
        raise SystemExit("export_map.toml names do not match original.exports.txt")
    if export_map.get("upstream_soname") != soname:
        raise SystemExit("export_map.toml upstream_soname does not match original.soname.txt")
    if any(entry.get("owning_phase", 0) > 3 for entry in symbols):
        raise SystemExit("export_map.toml still contains entries above owning_phase 3")
    raise SystemExit(0)

objdump_output = run("objdump", "-T", str(upstream_so)).splitlines()
exports = []
line_re = re.compile(
    r"^(?P<value>[0-9a-fA-F]+)\s+(?P<bind>\S+)\s+(?P<type>\S+)\s+(?P<section>\S+)\s+"
    r"(?P<size>[0-9a-fA-F]+)\s+(?P<version>\S+)\s+(?P<name>\S+)$"
)
for line in objdump_output:
    match = line_re.match(line.strip())
    if not match:
        continue
    name = match.group("name")
    if not name.startswith(("ZSTD_", "ZDICT_")):
        continue
    if match.group("section").startswith("*"):
        continue
    exports.append(match.groupdict())

if not exports:
    raise SystemExit("failed to extract public exports from upstream shared object")

readelf_dynamic = run("readelf", "-d", str(upstream_so))
soname_match = re.search(r"Library soname: \[(?P<soname>[^\]]+)\]", readelf_dynamic)
if not soname_match:
    raise SystemExit("failed to extract SONAME from upstream shared object")
soname = soname_match.group("soname")

header_paths = {
    "zstd.h": repo_root / "original/libzstd-1.5.5+dfsg2/lib/zstd.h",
    "zdict.h": repo_root / "original/libzstd-1.5.5+dfsg2/lib/zdict.h",
    "zstd_errors.h": repo_root / "original/libzstd-1.5.5+dfsg2/lib/zstd_errors.h",
}
header_text = {
    name: strip_comments(path.read_text(encoding="utf-8"))
    for name, path in header_paths.items()
}


def find_header(symbol: str) -> str:
    pattern = re.compile(rf"\b{re.escape(symbol)}\s*\(")
    for header in ("zstd_errors.h", "zdict.h", "zstd.h"):
        if pattern.search(header_text[header]):
            return header
    raise SystemExit(f"failed to map symbol {symbol} to a public header")


exports_text_lines = [
    "# source: original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5",
    "# format: name<TAB>bind<TAB>type_class<TAB>section<TAB>version_class<TAB>size_hex<TAB>value_hex",
]

export_map_lines = [
    'schema_version = 1',
    'upstream_shared_object = "original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5"',
    f'upstream_soname = "{soname}"',
    "",
]

existing_status = {}
if export_map_out.exists():
    try:
        existing = tomllib.loads(export_map_out.read_text(encoding="utf-8"))
    except Exception:
        existing = {"symbol": []}
    existing_status = {
        entry["name"]: entry.get("status", "planned")
        for entry in existing.get("symbol", [])
    }

for export in exports:
    name = export["name"]
    header = find_header(name)
    phase = classify_phase(name)
    exports_text_lines.append(
        "\t".join(
            [
                name,
                export["bind"],
                export["type"],
                export["section"],
                export["version"],
                f"0x{export['size']}",
                f"0x{export['value']}",
            ]
        )
    )
    export_map_lines.extend(
        [
            "[[symbol]]",
            f'name = "{name}"',
            f'binding = "{export["bind"]}"',
            f'type_class = "{export["type"]}"',
            f'version_class = "{export["version"]}"',
            f'originating_header = "{header}"',
            f'owner_module = "{owner_module(name)}"',
            f"owning_phase = {phase}",
            f'status = "{existing_status.get(name, "planned")}"',
            "",
        ]
    )

exports_text = "\n".join(exports_text_lines) + "\n"
soname_text = soname + "\n"
export_map_text = "\n".join(export_map_lines)

for path, expected in (
    (exports_out, exports_text),
    (soname_out, soname_text),
    (export_map_out, export_map_text),
):
    if mode == "check":
        actual = path.read_text(encoding="utf-8")
        if actual != expected:
            raise SystemExit(f"{path} is out of date; run safe/scripts/capture-upstream-abi.sh")
    else:
        path.write_text(expected, encoding="utf-8")
PY
