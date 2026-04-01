#![allow(dead_code)]
#![allow(non_camel_case_types)]

use crate::{
    compress::strategies::{double_fast, fast, lazy, opt},
    ffi::types::{ZSTD_CCtx, ZSTD_paramSwitch_e, ZSTD_strategy},
};
use core::ffi::c_void;

#[repr(C)]
#[derive(Default)]
pub(crate) struct MatchState {
    pub(crate) next_index: u32,
    pub(crate) loaded_bytes: usize,
    pub(crate) tree_updates: usize,
}

#[repr(C)]
#[derive(Default)]
pub(crate) struct SeqStore {
    pub(crate) count: usize,
    pub(crate) literal_bytes: usize,
    pub(crate) match_bytes: usize,
    pub(crate) last_offset: u32,
}

#[repr(C)]
#[derive(Default)]
pub(crate) struct CompressedBlockState {
    pub(crate) valid: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RawSequence {
    pub offset: u32,
    pub litLength: u32,
    pub matchLength: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RawSeqStore {
    pub seq: *mut RawSequence,
    pub pos: usize,
    pub posInSequence: usize,
    pub size: usize,
    pub capacity: usize,
}

pub(crate) type BlockRepcodes = [u32; 3];

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DictTableLoadMethod {
    Fast = 0,
    Full = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TableFillPurpose {
    ForCCtx = 0,
    ForCDict = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DictMode {
    NoDict = 0,
    ExtDict = 1,
    DictMatchState = 2,
    DedicatedDictSearch = 3,
}

pub(crate) type BlockCompressor =
    unsafe fn(*mut MatchState, *mut SeqStore, *mut u32, *const c_void, usize) -> usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StrategyAnalysisProfile {
    pub min_match: usize,
    pub preferred_distance: usize,
    pub search_depth: usize,
    pub hash_bits: u32,
}

pub(crate) fn get_seq_store(_ctx: *const ZSTD_CCtx) -> *const SeqStore {
    core::ptr::null()
}

pub(crate) fn reset_compressed_block_state(state: *mut CompressedBlockState) {
    if let Some(state) = unsafe { state.as_mut() } {
        *state = CompressedBlockState::default();
    }
}

pub(crate) fn reset_seq_store(seq_store: *mut SeqStore) {
    if let Some(seq_store) = unsafe { seq_store.as_mut() } {
        *seq_store = SeqStore::default();
    }
}

pub(crate) fn invalidate_rep_codes(_cctx: *mut ZSTD_CCtx) {}

pub(crate) fn insert_and_find_first_index(match_state: *mut MatchState, _input: *const u8) -> u32 {
    let Some(match_state) = (unsafe { match_state.as_mut() }) else {
        return 0;
    };
    let index = match_state.next_index;
    match_state.next_index = match_state.next_index.saturating_add(1);
    match_state.loaded_bytes = match_state.loaded_bytes.saturating_add(1);
    index
}

pub(crate) fn row_update(match_state: *mut MatchState, _input: *const u8) {
    if let Some(match_state) = unsafe { match_state.as_mut() } {
        match_state.next_index = match_state.next_index.saturating_add(1);
        match_state.loaded_bytes = match_state.loaded_bytes.saturating_add(1);
    }
}

fn run_length(src: &[u8], pos: usize) -> usize {
    let mut len = 1usize;
    while pos + len < src.len() && src[pos + len] == src[pos] {
        len += 1;
    }
    len
}

fn match_length_at_distance(src: &[u8], pos: usize, distance: usize) -> usize {
    if pos < distance {
        return 0;
    }

    let mut len = 0usize;
    while pos + len < src.len() && src[pos + len] == src[pos + len - distance] {
        len += 1;
    }
    len
}

fn match_length_between(src: &[u8], left: usize, right: usize) -> usize {
    let mut len = 0usize;
    while right + len < src.len() && src[left + len] == src[right + len] {
        len += 1;
    }
    len
}

fn hash_sequence(src: &[u8], pos: usize, hash_bits: u32) -> Option<usize> {
    if pos + 4 > src.len() {
        return None;
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&src[pos..pos + 4]);
    let value = u32::from_le_bytes(bytes).wrapping_mul(2_654_435_761);
    let shift = u32::BITS.saturating_sub(hash_bits.min(20));
    Some((value >> shift) as usize)
}

fn remember_match_candidate(
    heads: &mut [u32],
    prev: &mut [u32],
    src: &[u8],
    pos: usize,
    hash_bits: u32,
) {
    const INVALID_POS: u32 = u32::MAX;

    let Some(hash) = hash_sequence(src, pos, hash_bits) else {
        return;
    };
    prev[pos] = heads[hash];
    heads[hash] = pos.try_into().unwrap_or(INVALID_POS);
}

fn longest_hash_match(
    src: &[u8],
    pos: usize,
    heads: &[u32],
    prev: &[u32],
    profile: StrategyAnalysisProfile,
) -> Option<(usize, usize)> {
    const INVALID_POS: u32 = u32::MAX;

    let hash = hash_sequence(src, pos, profile.hash_bits)?;
    let mut candidate = heads[hash];
    let mut best = None;
    let mut searched = 0usize;

    while candidate != INVALID_POS && searched < profile.search_depth {
        let candidate_pos = candidate as usize;
        if candidate_pos >= pos {
            break;
        }
        let distance = pos - candidate_pos;
        let len = match_length_between(src, candidate_pos, pos);
        if len >= profile.min_match && best.is_none_or(|(best_len, _)| len > best_len) {
            best = Some((len, distance));
        }
        candidate = prev[candidate_pos];
        searched += 1;
    }

    best
}

fn best_repeated_match(
    src: &[u8],
    pos: usize,
    repcodes: &[u32; 3],
    profile: StrategyAnalysisProfile,
) -> Option<(usize, usize)> {
    let mut best = None;
    for &distance in repcodes
        .iter()
        .filter(|distance| **distance != 0)
        .chain(core::iter::once(&(profile.preferred_distance as u32)))
    {
        let distance = distance as usize;
        if pos < distance {
            continue;
        }
        let len = match_length_at_distance(src, pos, distance);
        if len >= profile.min_match && best.is_none_or(|(best_len, _)| len > best_len) {
            best = Some((len, distance));
        }
    }
    best
}

pub(crate) unsafe fn analyze_block_with_profile(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
    profile: StrategyAnalysisProfile,
) -> usize {
    const INVALID_POS: u32 = u32::MAX;

    if src.is_null() || src_size == 0 {
        return 0;
    }

    let src = unsafe { core::slice::from_raw_parts(src.cast::<u8>(), src_size) };
    let profile = StrategyAnalysisProfile {
        min_match: profile.min_match.max(3),
        preferred_distance: profile.preferred_distance.max(1),
        search_depth: profile.search_depth.max(1),
        hash_bits: profile.hash_bits.clamp(8, 20),
    };
    let mut heads = vec![INVALID_POS; 1usize << profile.hash_bits];
    let mut prev = vec![INVALID_POS; src.len()];
    let mut active_repcodes = if repcodes.is_null() {
        [1, 4, 8]
    } else {
        let repcodes = unsafe { core::slice::from_raw_parts(repcodes, 3) };
        [repcodes[0].max(1), repcodes[1].max(1), repcodes[2].max(1)]
    };
    let mut pos = 0usize;
    let mut sequences = 0usize;
    let mut literal_bytes = 0usize;
    let mut match_bytes = 0usize;
    let mut pending_literals = 0usize;
    let mut last_offset = 1u32;

    while pos + profile.min_match <= src.len() {
        let rle_len = run_length(src, pos);
        let mut best = if rle_len >= profile.min_match {
            Some((rle_len, 1usize))
        } else {
            None
        };
        if let Some(candidate) = best_repeated_match(src, pos, &active_repcodes, profile) {
            if best.is_none_or(|(best_len, _)| candidate.0 > best_len) {
                best = Some(candidate);
            }
        }
        if let Some(candidate) = longest_hash_match(src, pos, &heads, &prev, profile) {
            if best.is_none_or(|(best_len, _)| candidate.0 > best_len) {
                best = Some(candidate);
            }
        }

        if let Some((len, distance)) = best {
            sequences += 1;
            literal_bytes += pending_literals;
            pending_literals = 0;
            match_bytes += len;
            last_offset = distance as u32;
            active_repcodes = [last_offset, active_repcodes[0], active_repcodes[1]];
            for insert_pos in pos..(pos + len).min(src.len()) {
                remember_match_candidate(&mut heads, &mut prev, src, insert_pos, profile.hash_bits);
            }
            pos += len;
            continue;
        }

        remember_match_candidate(&mut heads, &mut prev, src, pos, profile.hash_bits);
        pending_literals += 1;
        pos += 1;
    }
    literal_bytes += pending_literals + src.len().saturating_sub(pos);

    if let Some(match_state) = unsafe { match_state.as_mut() } {
        match_state.next_index = match_state.next_index.saturating_add(src_size as u32);
        match_state.loaded_bytes = match_state.loaded_bytes.saturating_add(src_size);
    }

    if let Some(seq_store) = unsafe { seq_store.as_mut() } {
        seq_store.count = seq_store.count.saturating_add(sequences);
        seq_store.literal_bytes = seq_store.literal_bytes.saturating_add(literal_bytes);
        seq_store.match_bytes = seq_store.match_bytes.saturating_add(match_bytes);
        seq_store.last_offset = last_offset;
    }

    if !repcodes.is_null() {
        let repcodes = unsafe { core::slice::from_raw_parts_mut(repcodes, 3) };
        repcodes.copy_from_slice(&active_repcodes);
    }

    match_bytes
}

pub(crate) unsafe fn analyze_block(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
    min_run: usize,
    repeat_distance: usize,
) -> usize {
    let search_depth = if min_run >= 7 {
        24
    } else if min_run >= 5 {
        16
    } else {
        8
    };
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: min_run,
                preferred_distance: repeat_distance,
                search_depth,
                hash_bits: 15,
            },
        )
    }
}

pub(crate) fn select_block_compressor(
    strategy: ZSTD_strategy,
    _row_matchfinder_mode: ZSTD_paramSwitch_e,
    dict_mode: DictMode,
) -> Option<BlockCompressor> {
    Some(match (strategy, dict_mode) {
        (ZSTD_strategy::ZSTD_fast, DictMode::ExtDict) => fast::compress_block_fast_ext_dict,
        (ZSTD_strategy::ZSTD_fast, DictMode::DictMatchState) => {
            fast::compress_block_fast_dict_match_state
        }
        (ZSTD_strategy::ZSTD_fast, _) => fast::compress_block_fast,
        (ZSTD_strategy::ZSTD_dfast, DictMode::ExtDict) => {
            double_fast::compress_block_double_fast_ext_dict
        }
        (ZSTD_strategy::ZSTD_dfast, DictMode::DictMatchState) => {
            double_fast::compress_block_double_fast_dict_match_state
        }
        (ZSTD_strategy::ZSTD_dfast, _) => double_fast::compress_block_double_fast,
        (ZSTD_strategy::ZSTD_greedy, DictMode::ExtDict) => lazy::compress_block_greedy_ext_dict,
        (ZSTD_strategy::ZSTD_greedy, DictMode::DictMatchState) => {
            lazy::compress_block_greedy_dict_match_state
        }
        (ZSTD_strategy::ZSTD_greedy, _) => lazy::compress_block_greedy,
        (ZSTD_strategy::ZSTD_lazy, DictMode::ExtDict) => lazy::compress_block_lazy_ext_dict,
        (ZSTD_strategy::ZSTD_lazy, DictMode::DictMatchState) => {
            lazy::compress_block_lazy_dict_match_state
        }
        (ZSTD_strategy::ZSTD_lazy, _) => lazy::compress_block_lazy,
        (ZSTD_strategy::ZSTD_lazy2, DictMode::ExtDict) => lazy::compress_block_lazy2_ext_dict,
        (ZSTD_strategy::ZSTD_lazy2, DictMode::DictMatchState) => {
            lazy::compress_block_lazy2_dict_match_state
        }
        (ZSTD_strategy::ZSTD_lazy2, _) => lazy::compress_block_lazy2,
        (ZSTD_strategy::ZSTD_btlazy2, DictMode::ExtDict) => lazy::compress_block_btlazy2_ext_dict,
        (ZSTD_strategy::ZSTD_btlazy2, DictMode::DictMatchState) => {
            lazy::compress_block_btlazy2_dict_match_state
        }
        (ZSTD_strategy::ZSTD_btlazy2, _) => lazy::compress_block_btlazy2,
        (ZSTD_strategy::ZSTD_btopt, DictMode::ExtDict) => opt::compress_block_btopt_ext_dict,
        (ZSTD_strategy::ZSTD_btopt, DictMode::DictMatchState) => {
            opt::compress_block_btopt_dict_match_state
        }
        (ZSTD_strategy::ZSTD_btopt, _) => opt::compress_block_btopt,
        (ZSTD_strategy::ZSTD_btultra, DictMode::ExtDict) => opt::compress_block_btultra_ext_dict,
        (ZSTD_strategy::ZSTD_btultra, DictMode::DictMatchState) => {
            opt::compress_block_btultra_dict_match_state
        }
        (ZSTD_strategy::ZSTD_btultra, _) => opt::compress_block_btultra,
        (ZSTD_strategy::ZSTD_btultra2, _) => opt::compress_block_btultra2,
    })
}
