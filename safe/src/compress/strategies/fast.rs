#![allow(dead_code)]

use crate::compress::match_state::{
    analyze_block, DictTableLoadMethod, MatchState, SeqStore, TableFillPurpose,
};
use core::ffi::c_void;

pub(crate) fn fill_hash_table(
    match_state: *mut MatchState,
    _end: *const c_void,
    load_method: DictTableLoadMethod,
    purpose: TableFillPurpose,
) {
    let Some(match_state) = (unsafe { match_state.as_mut() }) else {
        return;
    };

    let step = match (load_method, purpose) {
        (DictTableLoadMethod::Fast, TableFillPurpose::ForCCtx) => 1usize,
        (DictTableLoadMethod::Fast, TableFillPurpose::ForCDict) => 2usize,
        (DictTableLoadMethod::Full, TableFillPurpose::ForCCtx) => 3usize,
        (DictTableLoadMethod::Full, TableFillPurpose::ForCDict) => 4usize,
    };
    match_state.next_index = match_state.next_index.saturating_add(step as u32);
    match_state.loaded_bytes = match_state.loaded_bytes.saturating_add(step);
}

pub(crate) unsafe fn compress_block_fast(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe { analyze_block(match_state, seq_store, repcodes, src, src_size, 4, 8) }
}

pub(crate) unsafe fn compress_block_fast_dict_match_state(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe { analyze_block(match_state, seq_store, repcodes, src, src_size, 4, 4) }
}

pub(crate) unsafe fn compress_block_fast_ext_dict(
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe { analyze_block(match_state, seq_store, repcodes, src, src_size, 4, 16) }
}
