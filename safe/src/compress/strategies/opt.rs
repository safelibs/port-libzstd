#![allow(dead_code)]

use crate::compress::match_state::{
    analyze_block_with_profile, MatchState, SeqStore, StrategyAnalysisProfile,
};
use core::ffi::c_void;

pub(crate) fn update_tree(match_state: *mut MatchState, input: *const u8, end: *const u8) {
    let Some(match_state) = (unsafe { match_state.as_mut() }) else {
        return;
    };
    let span = (end as usize).saturating_sub(input as usize);
    match_state.next_index = match_state.next_index.saturating_add(span as u32);
    match_state.tree_updates = match_state.tree_updates.saturating_add(span);
}

pub(crate) unsafe fn compress_block_btopt(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: 6,
                preferred_distance: 24,
                search_depth: 28,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btultra(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: 7,
                preferred_distance: 32,
                search_depth: 36,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btultra2(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: 8,
                preferred_distance: 48,
                search_depth: 48,
                hash_bits: 17,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btopt_dict_match_state(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: 6,
                preferred_distance: 20,
                search_depth: 28,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btultra_dict_match_state(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: 7,
                preferred_distance: 28,
                search_depth: 34,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btopt_ext_dict(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: 6,
                preferred_distance: 36,
                search_depth: 30,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btultra_ext_dict(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe {
        analyze_block_with_profile(
            match_state,
            seq_store,
            repcodes,
            src,
            src_size,
            StrategyAnalysisProfile {
                min_match: 7,
                preferred_distance: 44,
                search_depth: 36,
                hash_bits: 17,
            },
        )
    }
}
