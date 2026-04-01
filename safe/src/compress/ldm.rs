#![allow(dead_code)]
#![allow(non_camel_case_types)]

use crate::{
    compress::match_state::{MatchState, RawSeqStore, SeqStore},
    ffi::types::{ZSTD_compressionParameters, ZSTD_paramSwitch_e},
};

#[repr(C)]
#[derive(Default)]
pub(crate) struct LdmState {
    pub(crate) bytes_seen: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LdmParams {
    pub enableLdm: ZSTD_paramSwitch_e,
    pub hashLog: u32,
    pub bucketSizeLog: u32,
    pub minMatchLength: u32,
    pub hashRateLog: u32,
    pub windowLog: u32,
}

pub(crate) fn fill_hash_table(
    state: *mut LdmState,
    ip: *const u8,
    iend: *const u8,
    _params: *const LdmParams,
) {
    if let Some(state) = unsafe { state.as_mut() } {
        let start = ip as usize;
        let end = iend as usize;
        state.bytes_seen = end.saturating_sub(start);
    }
}

pub(crate) fn generate_sequences(
    _state: *mut LdmState,
    sequences: *mut RawSeqStore,
    params: *const LdmParams,
    src: *const core::ffi::c_void,
    src_size: usize,
) -> usize {
    let Some(sequences) = (unsafe { sequences.as_mut() }) else {
        return 0;
    };
    if sequences.seq.is_null() || sequences.capacity == 0 || src.is_null() || src_size == 0 {
        sequences.pos = 0;
        sequences.posInSequence = 0;
        sequences.size = 0;
        return 0;
    }

    let min_match = unsafe { params.as_ref() }
        .map(|params| params.minMatchLength.max(4) as usize)
        .unwrap_or(4);
    let src = unsafe { core::slice::from_raw_parts(src.cast::<u8>(), src_size) };
    let output = unsafe { core::slice::from_raw_parts_mut(sequences.seq, sequences.capacity) };
    let mut written = 0usize;
    let mut literal_start = 0usize;
    let mut pos = 0usize;

    while pos + min_match <= src.len() && written < output.len() {
        let mut run_len = 1usize;
        while pos + run_len < src.len() && src[pos + run_len] == src[pos] {
            run_len += 1;
        }

        if run_len >= min_match {
            output[written] = crate::compress::match_state::RawSequence {
                offset: 1,
                litLength: (pos - literal_start) as u32,
                matchLength: run_len as u32,
            };
            written += 1;
            pos += run_len;
            literal_start = pos;
        } else {
            pos += 1;
        }
    }

    sequences.pos = 0;
    sequences.posInSequence = 0;
    sequences.size = written;
    written
}

pub(crate) fn block_compress(
    raw_seq_store: *mut RawSeqStore,
    match_state: *mut MatchState,
    seq_store: *mut SeqStore,
    repcodes: *mut u32,
    _use_row_match_finder: ZSTD_paramSwitch_e,
    _src: *const core::ffi::c_void,
    src_size: usize,
) -> usize {
    let Some(raw_seq_store) = (unsafe { raw_seq_store.as_mut() }) else {
        return 0;
    };
    if raw_seq_store.seq.is_null() || raw_seq_store.size <= raw_seq_store.pos {
        return 0;
    }

    let raw_sequences =
        unsafe { core::slice::from_raw_parts(raw_seq_store.seq, raw_seq_store.size) };
    let remaining = &raw_sequences[raw_seq_store.pos..];
    let mut matched = 0usize;
    let mut last_offset = 1u32;

    if let Some(seq_store) = unsafe { seq_store.as_mut() } {
        for seq in remaining {
            seq_store.count = seq_store.count.saturating_add(1);
            seq_store.literal_bytes = seq_store
                .literal_bytes
                .saturating_add(seq.litLength as usize);
            seq_store.match_bytes = seq_store
                .match_bytes
                .saturating_add(seq.matchLength as usize);
            seq_store.last_offset = seq.offset.max(1);
            matched = matched.saturating_add(seq.matchLength as usize);
            last_offset = seq.offset.max(1);
        }
    } else {
        matched = remaining
            .iter()
            .map(|seq| seq.matchLength as usize)
            .sum::<usize>();
        last_offset = remaining.last().map(|seq| seq.offset.max(1)).unwrap_or(1);
    }

    raw_seq_store.pos = raw_seq_store.size;
    raw_seq_store.posInSequence = 0;

    if let Some(match_state) = unsafe { match_state.as_mut() } {
        match_state.next_index = match_state.next_index.saturating_add(src_size as u32);
        match_state.loaded_bytes = match_state.loaded_bytes.saturating_add(src_size);
    }

    if !repcodes.is_null() {
        let repcodes = unsafe { core::slice::from_raw_parts_mut(repcodes, 3) };
        repcodes[2] = repcodes[1];
        repcodes[1] = repcodes[0];
        repcodes[0] = last_offset;
    }

    matched
}

pub(crate) fn skip_sequences(raw_seq_store: *mut RawSeqStore, src_size: usize, _min_match: u32) {
    if let Some(raw_seq_store) = unsafe { raw_seq_store.as_mut() } {
        raw_seq_store.pos = raw_seq_store.pos.saturating_add(src_size);
    }
}

pub(crate) fn skip_raw_seq_store_bytes(raw_seq_store: *mut RawSeqStore, bytes: usize) {
    if let Some(raw_seq_store) = unsafe { raw_seq_store.as_mut() } {
        raw_seq_store.posInSequence = raw_seq_store.posInSequence.saturating_add(bytes);
    }
}

pub(crate) fn get_table_size(params: LdmParams) -> usize {
    if params.enableLdm == ZSTD_paramSwitch_e::ZSTD_ps_disable {
        0
    } else {
        1usize << params.hashLog.min(30)
    }
}

pub(crate) fn get_max_nb_seq(params: LdmParams, max_chunk_size: usize) -> usize {
    let min_match = params.minMatchLength.max(1) as usize;
    max_chunk_size / min_match + 1
}

pub(crate) fn adjust_parameters(
    params: *mut LdmParams,
    cparams: *const ZSTD_compressionParameters,
) {
    let Some(params) = (unsafe { params.as_mut() }) else {
        return;
    };
    let Some(cparams) = (unsafe { cparams.as_ref() }) else {
        return;
    };

    if params.windowLog == 0 {
        params.windowLog = cparams.windowLog;
    }
    if params.hashLog == 0 {
        params.hashLog = cparams.hashLog;
    }
    if params.minMatchLength == 0 {
        params.minMatchLength = cparams.minMatch;
    }
}
