#![allow(dead_code)]
#![allow(non_camel_case_types)]

use crate::{
    common::error::error_result,
    compress::match_state::SeqStore,
    ffi::types::{ZSTD_ErrorCode, ZSTD_strategy},
};
use core::ffi::c_int;

pub(crate) type FSE_CTable = u32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FSE_repeat {
    FSE_repeat_none = 0,
    FSE_repeat_check = 1,
    FSE_repeat_valid = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SymbolEncodingType {
    SetBasic = 0,
    SetRle = 1,
    SetCompressed = 2,
    SetRepeat = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DefaultPolicy {
    ZSTD_defaultDisallowed = 0,
    ZSTD_defaultAllowed = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SeqDef {
    pub offBase: u32,
    pub litLength: u16,
    pub mlBase: u16,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn select_encoding_type(
    repeat_mode: *mut FSE_repeat,
    count: *const u32,
    max: u32,
    most_frequent: usize,
    nb_seq: usize,
    _fse_log: u32,
    _prev_ctable: *const FSE_CTable,
    _default_norm: *const i16,
    _default_norm_log: u32,
    _default_allowed: DefaultPolicy,
    _strategy: ZSTD_strategy,
) -> SymbolEncodingType {
    if let Some(repeat_mode) = unsafe { repeat_mode.as_mut() } {
        *repeat_mode = if nb_seq == 0 || count.is_null() {
            FSE_repeat::FSE_repeat_none
        } else {
            FSE_repeat::FSE_repeat_valid
        };
    }

    if nb_seq == 0 || count.is_null() {
        SymbolEncodingType::SetBasic
    } else {
        let frequencies = unsafe { core::slice::from_raw_parts(count, max as usize + 1) };
        let non_zero = frequencies.iter().filter(|value| **value != 0).count();
        if most_frequent >= nb_seq.saturating_sub(1) {
            SymbolEncodingType::SetRle
        } else if non_zero <= 1 {
            SymbolEncodingType::SetRepeat
        } else {
            SymbolEncodingType::SetCompressed
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_sequences(
    _dst: *mut u8,
    _dst_capacity: usize,
    _match_length_ctable: *const FSE_CTable,
    _match_length_codes: *const u8,
    _offset_ctable: *const FSE_CTable,
    _offset_codes: *const u8,
    _literal_length_ctable: *const FSE_CTable,
    _literal_length_codes: *const u8,
    _sequences: *const SeqDef,
    _nb_seq: usize,
    _long_offsets: bool,
    _bmi2: bool,
) -> usize {
    if _nb_seq == 0 {
        return 0;
    }
    if _dst.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong);
    }
    if _sequences.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    }

    let seq_size = core::mem::size_of::<SeqDef>();
    let required = _nb_seq.saturating_mul(seq_size);
    if required > _dst_capacity {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    let dst = unsafe { core::slice::from_raw_parts_mut(_dst, required) };
    let sequences = unsafe { core::slice::from_raw_parts(_sequences, _nb_seq) };
    let mut written = 0usize;
    for seq in sequences {
        dst[written..written + 2].copy_from_slice(&seq.litLength.to_le_bytes());
        written += 2;
        dst[written..written + 2].copy_from_slice(&seq.mlBase.to_le_bytes());
        written += 2;
        dst[written..written + 4].copy_from_slice(&seq.offBase.to_le_bytes());
        written += 4;
    }
    written
}

pub(crate) fn fse_bit_cost(_ctable: *const FSE_CTable, count: *const u32, max: u32) -> usize {
    if count.is_null() {
        return 0;
    }
    let count = unsafe { core::slice::from_raw_parts(count, max as usize + 1) };
    count.iter().map(|value| usize::from(*value != 0)).sum()
}

pub(crate) fn seq_to_codes(seq_store: *const SeqStore) -> c_int {
    let Some(seq_store) = (unsafe { seq_store.as_ref() }) else {
        return 0;
    };
    seq_store.count.min(c_int::MAX as usize) as c_int
}
