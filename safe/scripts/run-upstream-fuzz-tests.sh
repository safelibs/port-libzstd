#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_export_safe_env

phase6_log "building ported fuzz drivers"
make -C "$SAFE_ROOT/tests/ported/whitebox" fuzz

WORK_DIR="$PHASE6_OUT/fuzz"
rm -rf "$WORK_DIR"
install -d "$WORK_DIR/corpora"

raw_targets=(
    block_round_trip
    decompress_dstSize_tooSmall
    dictionary_loader
    dictionary_round_trip
    dictionary_stream_round_trip
    fse_read_ncount
    huf_decompress
    huf_round_trip
    raw_dictionary_round_trip
    seekable_roundtrip
    sequence_compression_api
    simple_compress
    simple_round_trip
    stream_round_trip
)
compressed_targets=(
    block_decompress
    dictionary_decompress
    simple_decompress
    stream_decompress
    zstd_frame_info
)

stage_corpus() {
    local target=$1
    local dest=$2
    install -d "$dest"
    if [[ -d $ORIGINAL_ROOT/tests/fuzz/corpora/$target ]]; then
        rsync -a "$ORIGINAL_ROOT/tests/fuzz/corpora/$target/" "$dest/"
        return
    fi
    if [[ -d $ORIGINAL_ROOT/tests/fuzz/corpora/${target}-seed ]]; then
        rsync -a "$ORIGINAL_ROOT/tests/fuzz/corpora/${target}-seed/" "$dest/"
        return
    fi
    if printf '%s\n' "${raw_targets[@]}" | grep -qx "$target"; then
        rsync -a "$FUZZ_FIXTURE_ROOT/raw/" "$dest/"
        return
    fi
    if printf '%s\n' "${compressed_targets[@]}" | grep -qx "$target"; then
        rsync -a "$FUZZ_FIXTURE_ROOT/compressed/" "$dest/"
        return
    fi
    rsync -a "$FUZZ_FIXTURE_ROOT/dictionary/" "$dest/"
}

targets=(
    block_decompress
    block_round_trip
    decompress_dstSize_tooSmall
    dictionary_decompress
    dictionary_loader
    dictionary_round_trip
    dictionary_stream_round_trip
    fse_read_ncount
    huf_decompress
    huf_round_trip
    raw_dictionary_round_trip
    seekable_roundtrip
    sequence_compression_api
    simple_compress
    simple_decompress
    simple_round_trip
    stream_decompress
    stream_round_trip
    zstd_frame_info
)

target=
for target in "${targets[@]}"; do
    corpus_dir="$WORK_DIR/corpora/$target"
    stage_corpus "$target" "$corpus_dir"
    phase6_log "running fuzz corpus driver: $target"
    "$SAFE_ROOT/out/phase6/whitebox/fuzz/$target" "$corpus_dir"
done
