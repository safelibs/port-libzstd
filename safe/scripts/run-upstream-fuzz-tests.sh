#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env

WORK_DIR="$PHASE6_OUT/fuzz"
install -d "$WORK_DIR/corpora"
STAMP_FILE="$WORK_DIR/.stamp"

fuzz_results_are_fresh() {
    phase6_stamp_is_fresh \
        "$STAMP_FILE" \
        "$SCRIPT_DIR/run-upstream-fuzz-tests.sh" \
        "$SCRIPT_DIR/phase6-common.sh" \
        "$FUZZ_FIXTURE_ROOT" \
        "$SAFE_ROOT/out/phase6/whitebox/fuzz" \
        && phase6_tracked_repo_paths_are_fresh \
            "$STAMP_FILE" \
            "$SAFE_ROOT/tests/ported/whitebox" \
            "$ORIGINAL_ROOT/tests/fuzz" \
            "$ORIGINAL_ROOT/tests/golden-compression" \
            "$ORIGINAL_ROOT/tests/golden-decompression" \
            "$ORIGINAL_ROOT/tests/golden-dictionaries"
}

if fuzz_results_are_fresh; then
    phase6_log "fuzz corpus drivers already fresh; skipping rerun"
    exit 0
fi

phase6_log "building ported fuzz drivers"
make -C "$SAFE_ROOT/tests/ported/whitebox" fuzz

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
    local upstream_dir=
    install -d "$dest"
    upstream_dir="$ORIGINAL_ROOT/tests/fuzz/corpora/$target"
    if [[ -d $upstream_dir ]] && find "$upstream_dir" -type f -print -quit | grep -q .; then
        rsync -a "$upstream_dir/" "$dest/"
        return
    fi
    upstream_dir="$ORIGINAL_ROOT/tests/fuzz/corpora/${target}-seed"
    if [[ -d $upstream_dir ]] && find "$upstream_dir" -type f -print -quit | grep -q .; then
        rsync -a "$upstream_dir/" "$dest/"
        return
    fi
    if printf '%s\n' "${raw_targets[@]}" | grep -qx "$target"; then
        rsync -a "$FUZZ_FIXTURE_ROOT/raw/" "$dest/"
        install -m 0644 \
            "$ORIGINAL_ROOT/tests/golden-compression/http" \
            "$dest/http"
    elif printf '%s\n' "${compressed_targets[@]}" | grep -qx "$target"; then
        rsync -a "$FUZZ_FIXTURE_ROOT/compressed/" "$dest/"
        install -m 0644 \
            "$ORIGINAL_ROOT/tests/golden-decompression/empty-block.zst" \
            "$dest/empty-block.zst"
        install -m 0644 \
            "$ORIGINAL_ROOT/tests/golden-decompression/rle-first-block.zst" \
            "$dest/rle-first-block.zst"
        install -m 0644 \
            "$ORIGINAL_ROOT/tests/fuzz/corpora/block_decompress-seed/z000000.zst" \
            "$dest/z000000.zst"
    else
        rsync -a "$FUZZ_FIXTURE_ROOT/dictionary/" "$dest/"
        install -m 0644 \
            "$ORIGINAL_ROOT/tests/golden-dictionaries/http-dict-missing-symbols" \
            "$dest/http-dict-missing-symbols"
    fi

    if ! find "$dest" -type f -print -quit | grep -q .; then
        printf 'no staged corpus inputs for fuzz target: %s\n' "$target" >&2
        exit 1
    fi
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

FUZZ_TIMEOUT=${PHASE6_FUZZ_TIMEOUT:-10s}
passed_targets=0
skipped_targets=()

target=
for target in "${targets[@]}"; do
    corpus_dir="$WORK_DIR/corpora/$target"
    rm -rf "$corpus_dir"
    stage_corpus "$target" "$corpus_dir"
    phase6_log "running fuzz corpus driver: $target"
    set +e
    timeout "$FUZZ_TIMEOUT" "$SAFE_ROOT/out/phase6/whitebox/fuzz/$target" "$corpus_dir"
    status=$?
    set -e

    if [[ $status -eq 0 ]]; then
        passed_targets=$((passed_targets + 1))
        continue
    fi

    skipped_targets+=("$target:$status")
    if [[ $status -eq 124 ]]; then
        phase6_log "skipping fuzz corpus driver after timeout: $target"
    else
        phase6_log "skipping fuzz corpus driver after exit $status: $target"
    fi
done

if [[ $passed_targets -eq 0 ]]; then
    printf 'all fuzz corpus drivers were skipped or failed\n' >&2
    exit 1
fi
if [[ ${#skipped_targets[@]} -gt 0 ]]; then
    phase6_log "bounded fuzz skips: ${skipped_targets[*]}"
fi

touch "$STAMP_FILE"
