#![allow(dead_code)]

use crate::compress::match_state::{
    analyze_block_with_profile, MatchState, SeqStore, StrategyAnalysisProfile,
};
use core::ffi::c_void;

pub(crate) unsafe fn compress_block_btlazy2(
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
                preferred_distance: 32,
                search_depth: 24,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_lazy2(
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
                min_match: 5,
                preferred_distance: 24,
                search_depth: 18,
                hash_bits: 15,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_lazy(
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
                min_match: 5,
                preferred_distance: 16,
                search_depth: 14,
                hash_bits: 15,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_greedy(
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
                min_match: 4,
                preferred_distance: 8,
                search_depth: 10,
                hash_bits: 14,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btlazy2_dict_match_state(
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
                search_depth: 22,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_lazy2_dict_match_state(
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
                min_match: 5,
                preferred_distance: 20,
                search_depth: 18,
                hash_bits: 15,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_lazy_dict_match_state(
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
                min_match: 5,
                preferred_distance: 12,
                search_depth: 14,
                hash_bits: 15,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_greedy_dict_match_state(
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
                min_match: 4,
                preferred_distance: 6,
                search_depth: 10,
                hash_bits: 14,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_btlazy2_ext_dict(
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
                preferred_distance: 40,
                search_depth: 26,
                hash_bits: 16,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_lazy2_ext_dict(
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
                min_match: 5,
                preferred_distance: 28,
                search_depth: 20,
                hash_bits: 15,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_lazy_ext_dict(
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
                min_match: 5,
                preferred_distance: 18,
                search_depth: 16,
                hash_bits: 15,
            },
        )
    }
}

pub(crate) unsafe fn compress_block_greedy_ext_dict(
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
                min_match: 4,
                preferred_distance: 10,
                search_depth: 12,
                hash_bits: 14,
            },
        )
    }
}
