use crate::{
    common::error::{error_result, is_error_result},
    decompress::huf::DICTIONARY_MAGIC,
    dict_builder::divsufsort,
    ffi::types::{ZDICT_legacy_params_t, ZDICT_params_t, ZSTD_ErrorCode},
};
use core::{
    cmp::Ordering,
    ffi::{c_char, c_uint, c_void},
    mem::size_of,
};
use oxiarc_zstd::{LevelConfig as OxiarcLevelConfig, MatchFinder as OxiarcMatchFinder};
use std::vec::Vec;

const FALLBACK_ENTROPY_TABLES: [u8; 138] = [
    0x47, 0x30, 0x26, 0x50, 0xdc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0x00, 0x17,
    0xef, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x36, 0xfb, 0xb4, 0xb4, 0xb6, 0x53, 0x12, 0xc9,
    0x4c, 0x11, 0x89, 0x90, 0x95, 0x99, 0x52, 0xd8, 0xe3, 0x6d, 0x49, 0x52, 0x25, 0xb7, 0xad, 0x05,
    0x60, 0x84, 0x91, 0xb5, 0x75, 0x4d, 0x95, 0xec, 0x16, 0xd9, 0xdd, 0x9b, 0x94, 0x12, 0x1c, 0xa9,
    0x09, 0x33, 0x4b, 0x00, 0xa0, 0x60, 0x32, 0x3c, 0x93, 0x11, 0x60, 0x90, 0xb0, 0x60, 0x36, 0xa2,
    0x11, 0x2a, 0x3b, 0x14, 0x00, 0x3a, 0x44, 0x2c, 0x1e, 0x2c, 0x21, 0xce, 0x24, 0xc2, 0x50, 0x10,
    0xe4, 0x30, 0x0a, 0x83, 0x20, 0x08, 0x62, 0x90, 0x31, 0xc6, 0x18, 0x62, 0x08, 0x32, 0x86, 0x18,
    0xa3, 0x54, 0x11, 0x51, 0x00, 0x74, 0x0c, 0xa8, 0xc4, 0xa3, 0xa9, 0x48, 0x1e, 0x8d, 0x92, 0x28,
    0x89, 0x51, 0x18, 0x74, 0x4a, 0x31, 0x65, 0x54, 0x04, 0x00,
];
const DEFAULT_REPCODES: [u32; 3] = [1, 4, 8];
pub(crate) const TRAINED_DICT_HEADER_SIZE: usize = 8 + 256 + size_of::<[u32; 3]>();
const MIN_DICT_CONTENT_SIZE: usize = 8;
const MIN_TOTAL_SAMPLE_BYTES: usize = 64;
const MIN_TRAINING_SAMPLES: usize = 8;
const MAX_OFFCODE: usize = 30;
const MAX_LITERAL_LENGTH_CODE: usize = 35;
const MAX_MATCH_LENGTH_CODE: usize = 52;
const MAX_HUFFMAN_DIRECT_WEIGHTS: usize = 128;

struct DictBitReader<'a> {
    idx: usize,
    source: &'a [u8],
}

impl<'a> DictBitReader<'a> {
    fn new(source: &'a [u8]) -> Self {
        Self { idx: 0, source }
    }

    fn bits_left(&self) -> usize {
        self.source.len().saturating_mul(8).saturating_sub(self.idx)
    }

    fn bits_read(&self) -> usize {
        self.idx
    }

    fn return_bits(&mut self, n: usize) -> Result<(), ZSTD_ErrorCode> {
        if n > self.idx {
            return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
        }
        self.idx -= n;
        Ok(())
    }

    fn get_bits(&mut self, n: usize) -> Result<u32, ZSTD_ErrorCode> {
        if n > 32 || self.bits_left() < n {
            return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
        }

        let bits_left_in_current_byte = 8 - (self.idx % 8);
        let bits_not_needed_in_current_byte = 8 - bits_left_in_current_byte;
        let mut value = u32::from(self.source[self.idx / 8] >> bits_not_needed_in_current_byte);

        if bits_left_in_current_byte >= n {
            value &= (1u32 << n) - 1;
            self.idx += n;
        } else {
            self.idx += bits_left_in_current_byte;
            let full_bytes_needed = (n - bits_left_in_current_byte) / 8;
            let bits_in_last_byte_needed = n - bits_left_in_current_byte - full_bytes_needed * 8;
            let mut bit_shift = bits_left_in_current_byte;

            for _ in 0..full_bytes_needed {
                value |= u32::from(self.source[self.idx / 8]) << bit_shift;
                self.idx += 8;
                bit_shift += 8;
            }

            if bits_in_last_byte_needed > 0 {
                let last_byte =
                    u32::from(self.source[self.idx / 8]) & ((1u32 << bits_in_last_byte_needed) - 1);
                value |= last_byte << bit_shift;
                self.idx += bits_in_last_byte_needed;
            }
        }

        Ok(value)
    }
}

fn src_slice<'a>(ptr: *const c_void, len: usize) -> Result<&'a [u8], ZSTD_ErrorCode> {
    if ptr.is_null() {
        return if len == 0 {
            Ok(&[])
        } else {
            Err(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong)
        };
    }
    Ok(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) })
}

fn dst_slice_mut<'a>(ptr: *mut c_void, len: usize) -> Result<&'a mut [u8], ZSTD_ErrorCode> {
    if ptr.is_null() {
        return if len == 0 {
            Ok(&mut [])
        } else {
            Err(ZSTD_ErrorCode::ZSTD_error_dstBuffer_null)
        };
    }
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr.cast::<u8>(), len) })
}

fn highest_bit_set(x: u32) -> u32 {
    debug_assert!(x > 0);
    u32::BITS - x.leading_zeros()
}

fn parse_huf_table_size(source: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    let Some(&header) = source.first() else {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
    };
    let size = if header < 128 {
        1 + header as usize
    } else {
        let num_weights = (header - 127) as usize;
        1 + num_weights.div_ceil(2)
    };
    if size > source.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
    }
    Ok(size)
}

fn parse_fse_table_size(source: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    const ACC_LOG_OFFSET: u8 = 5;

    let mut reader = DictBitReader::new(source);
    let accuracy_log = ACC_LOG_OFFSET + reader.get_bits(4)? as u8;
    let probability_sum = 1u32
        .checked_shl(accuracy_log.into())
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted)?;
    let mut probability_counter = 0u32;
    let mut symbols = 0usize;

    while probability_counter < probability_sum {
        let max_remaining_value = probability_sum - probability_counter + 1;
        let bits_to_read = highest_bit_set(max_remaining_value) as usize;
        let unchecked_value = reader.get_bits(bits_to_read)?;
        let low_threshold = ((1u32 << bits_to_read) - 1).saturating_sub(max_remaining_value);
        let mask = (1u32 << (bits_to_read - 1)) - 1;
        let small_value = unchecked_value & mask;
        let value = if small_value < low_threshold {
            reader.return_bits(1)?;
            small_value
        } else if unchecked_value > mask {
            unchecked_value - low_threshold
        } else {
            unchecked_value
        };
        let prob = value as i32 - 1;
        symbols += 1;

        if prob > 0 {
            probability_counter += prob as u32;
        } else if prob == -1 {
            probability_counter += 1;
        } else {
            loop {
                let skip_amount = reader.get_bits(2)? as usize;
                symbols += skip_amount;
                if skip_amount != 3 {
                    break;
                }
            }
        }

        if symbols > 256 {
            return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
        }
    }

    Ok(reader.bits_read().div_ceil(8))
}

pub(crate) fn formatted_dictionary_content(bytes: &[u8]) -> Result<&[u8], ZSTD_ErrorCode> {
    if bytes.len() < 8 || bytes[..4] != DICTIONARY_MAGIC {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
    }

    let mut pos = 8usize;
    pos += parse_huf_table_size(&bytes[pos..])?;
    pos += parse_fse_table_size(&bytes[pos..])?;
    pos += parse_fse_table_size(&bytes[pos..])?;
    pos += parse_fse_table_size(&bytes[pos..])?;

    if pos + 12 > bytes.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
    }

    let dict_content = &bytes[pos + 12..];
    for chunk in bytes[pos..pos + 12].chunks_exact(4) {
        let rep = u32::from_le_bytes(chunk.try_into().expect("repcode chunk is 4 bytes"));
        if rep == 0 || rep as usize > dict_content.len() {
            return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
        }
    }
    Ok(dict_content)
}

pub(crate) fn dictionary_header_size(bytes: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    let content = formatted_dictionary_content(bytes)?;
    Ok(bytes.len() - content.len())
}

pub(crate) fn parse_samples<'a>(
    samples_buffer: *const c_void,
    samples_sizes: *const usize,
    nb_samples: c_uint,
) -> Result<Vec<&'a [u8]>, ZSTD_ErrorCode> {
    let nb_samples = nb_samples as usize;
    if nb_samples == 0 || samples_sizes.is_null() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed);
    }

    let sample_sizes = unsafe { core::slice::from_raw_parts(samples_sizes, nb_samples) };
    let total_size = sample_sizes
        .iter()
        .try_fold(0usize, |total, size| total.checked_add(*size))
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed)?;
    let buffer = src_slice(samples_buffer, total_size)?;

    let mut offset = 0usize;
    let mut samples = Vec::with_capacity(nb_samples);
    for &size in sample_sizes {
        let end = offset
            .checked_add(size)
            .ok_or(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed)?;
        if end > buffer.len() {
            return Err(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed);
        }
        samples.push(&buffer[offset..end]);
        offset = end;
    }

    if samples.iter().map(|sample| sample.len()).sum::<usize>() < MIN_TOTAL_SAMPLE_BYTES
        || !samples
            .iter()
            .any(|sample| sample.len() >= MIN_DICT_CONTENT_SIZE)
    {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed);
    }

    Ok(samples)
}

#[derive(Default)]
struct HeaderBitWriter {
    output: Vec<u8>,
    partial: u64,
    bits_in_partial: usize,
}

impl HeaderBitWriter {
    fn write_bits(&mut self, bits: u64, num_bits: usize) {
        if num_bits == 0 {
            return;
        }
        if self.bits_in_partial + num_bits < 64 {
            self.partial |= bits << self.bits_in_partial;
            self.bits_in_partial += num_bits;
            return;
        }

        let bits_free = 64 - self.bits_in_partial;
        self.partial |= bits << self.bits_in_partial;
        self.output.extend_from_slice(&self.partial.to_le_bytes());
        self.partial = 0;
        self.bits_in_partial = 0;

        let mut remaining_bits = num_bits - bits_free;
        let mut remaining = bits >> bits_free;
        while remaining_bits >= 8 {
            self.output.push(remaining as u8);
            remaining >>= 8;
            remaining_bits -= 8;
        }
        if remaining_bits > 0 {
            self.partial = remaining & ((1u64 << remaining_bits) - 1);
            self.bits_in_partial = remaining_bits;
        }
    }

    fn align_to_byte(&mut self) {
        if self.bits_in_partial == 0 {
            return;
        }
        let full_bytes = self.bits_in_partial.div_ceil(8);
        self.output
            .extend_from_slice(&self.partial.to_le_bytes()[..full_bytes]);
        self.partial = 0;
        self.bits_in_partial = 0;
    }

    fn finish(mut self) -> Vec<u8> {
        self.align_to_byte();
        self.output
    }
}

#[derive(Clone)]
struct FseHeader {
    probabilities: Vec<i32>,
    acc_log: u8,
}

struct EntropyStats {
    literal_counts: [usize; 256],
    offcode_counts: [usize; MAX_OFFCODE + 1],
    match_length_counts: [usize; MAX_MATCH_LENGTH_CODE + 1],
    lit_length_counts: [usize; MAX_LITERAL_LENGTH_CODE + 1],
}

impl Default for EntropyStats {
    fn default() -> Self {
        Self {
            literal_counts: [0; 256],
            offcode_counts: [0; MAX_OFFCODE + 1],
            match_length_counts: [0; MAX_MATCH_LENGTH_CODE + 1],
            lit_length_counts: [0; MAX_LITERAL_LENGTH_CODE + 1],
        }
    }
}

impl EntropyStats {
    fn new() -> Self {
        let mut stats = Self::default();
        stats.offcode_counts.fill(1);
        stats.match_length_counts.fill(1);
        stats.lit_length_counts.fill(1);
        stats
    }
}

#[derive(Clone)]
struct SimpleHuffmanTable {
    codes: Vec<(u32, u8)>,
}

impl SimpleHuffmanTable {
    fn build_from_counts(counts: &[usize]) -> Self {
        assert!(counts.len() <= 256);
        let non_zero = counts.iter().filter(|count| **count != 0).count().max(2);
        let mut weights = distribute_weights(non_zero);
        let max_bits = weights.len().ilog2() as usize + 2;
        redistribute_weights(&mut weights, max_bits);
        weights.reverse();

        let mut counts_sorted = counts.iter().enumerate().collect::<Vec<_>>();
        counts_sorted.sort_by_key(|(idx, count)| (**count, *idx));

        let mut distributed = vec![0usize; counts.len()];
        for (idx, count) in counts_sorted {
            if *count != 0 {
                distributed[idx] = weights.pop().unwrap_or(1);
            }
        }

        Self::build_from_weights(&distributed)
    }

    fn build_from_weights(weights: &[usize]) -> Self {
        #[derive(Clone, Copy)]
        struct Entry {
            symbol: u8,
            weight: usize,
        }

        let mut sorted = Vec::new();
        for (symbol, weight) in weights.iter().copied().enumerate() {
            if weight != 0 {
                sorted.push(Entry {
                    symbol: symbol as u8,
                    weight,
                });
            }
        }
        sorted.sort_by(|left, right| match left.weight.cmp(&right.weight) {
            Ordering::Equal => left.symbol.cmp(&right.symbol),
            order => order,
        });

        let mut table = Self {
            codes: vec![(0, 0); weights.len()],
        };
        let weight_sum = sorted
            .iter()
            .map(|entry| 1usize << (entry.weight - 1))
            .sum::<usize>();
        let max_num_bits = highest_bit_set_usize(weight_sum) - 1;
        let mut current_code = 0usize;
        let mut current_weight = 0usize;
        let mut current_num_bits = 0usize;

        for entry in &sorted {
            if current_weight != entry.weight {
                current_code >>= entry.weight - current_weight;
                current_num_bits = max_num_bits - entry.weight + 1;
                current_weight = entry.weight;
            }
            table.codes[entry.symbol as usize] = (current_code as u32, current_num_bits as u8);
            current_code += 1;
        }

        table
    }

    fn weights(&self) -> Vec<u8> {
        let max_bits = self
            .codes
            .iter()
            .map(|(_, num_bits)| *num_bits)
            .max()
            .unwrap_or(0);
        self.codes
            .iter()
            .map(|(_, num_bits)| {
                if *num_bits == 0 {
                    0
                } else {
                    max_bits - *num_bits + 1
                }
            })
            .collect()
    }
}

fn highest_bit_set_usize(x: usize) -> usize {
    debug_assert!(x > 0);
    usize::BITS as usize - x.leading_zeros() as usize
}

fn distribute_weights(amount: usize) -> Vec<usize> {
    let amount = amount.clamp(2, 256);
    let mut weights = vec![1, 1];
    let mut target_weight = 1usize;
    let mut weight_counter = 2usize;

    while weights.len() < amount {
        let mut add_new = 1usize << (weight_counter - target_weight);
        let available_space = amount - weights.len();
        if add_new > available_space {
            target_weight = weight_counter;
            add_new = 1;
        }
        for _ in 0..add_new {
            weights.push(target_weight);
        }
        weight_counter += 1;
    }

    weights
}

fn redistribute_weights(weights: &mut [usize], max_num_bits: usize) {
    let weight_sum_log = weights
        .iter()
        .map(|weight| 1usize << *weight)
        .sum::<usize>()
        .ilog2() as usize;
    if weight_sum_log < max_num_bits {
        return;
    }

    let decrease_by = weight_sum_log - max_num_bits + 1;
    let mut added_weight = 0usize;
    for weight in weights.iter_mut() {
        if *weight < decrease_by {
            for add in *weight..decrease_by {
                added_weight += 1usize << add;
            }
            *weight = decrease_by;
        }
    }

    while added_weight > 0 {
        let mut current_idx = 0usize;
        let mut current_weight = 0usize;
        for (idx, weight) in weights.iter().copied().enumerate() {
            if (1usize << (weight - 1)) > added_weight {
                break;
            }
            if weight > current_weight {
                current_weight = weight;
                current_idx = idx;
            }
        }
        added_weight -= 1usize << (current_weight - 1);
        weights[current_idx] -= 1;
    }

    if weights[0] > 1 {
        let offset = weights[0] - 1;
        for weight in weights.iter_mut() {
            *weight -= offset;
        }
    }
}

fn build_huffman_table(counts: &[usize; 256]) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let max_symbol = counts
        .iter()
        .rposition(|count| *count != 0)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed)?
        .max(1);
    let table = SimpleHuffmanTable::build_from_counts(&counts[..=max_symbol]);
    let mut weights = table.weights();
    if weights.len() <= 1 {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed);
    }
    weights.pop();
    if weights.len() > MAX_HUFFMAN_DIRECT_WEIGHTS {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed);
    }

    let mut output = Vec::with_capacity(1 + weights.len().div_ceil(2));
    output.push((127 + weights.len()) as u8);
    for pair in weights.chunks(2) {
        let high = pair[0] & 0x0F;
        let low = pair.get(1).copied().unwrap_or(0) & 0x0F;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn build_fse_header(counts: &[usize], max_log: u8, avoid_0_numbit: bool) -> FseHeader {
    let mut probabilities = counts.iter().map(|count| *count as i32).collect::<Vec<_>>();
    let mut min_count = 0usize;
    for count in counts {
        if *count > 0 && (min_count == 0 || *count < min_count) {
            min_count = *count;
        }
    }
    let shift = min_count.saturating_sub(1) as i32;
    let mut max_prob = 0i32;
    for probability in &mut probabilities {
        if *probability > 0 {
            *probability -= shift;
        }
        max_prob = max_prob.max(*probability);
    }

    if max_prob > 0 && max_prob as usize > probabilities.len() {
        let divisor = max_prob / probabilities.len() as i32;
        for probability in &mut probabilities {
            if *probability > 0 {
                *probability = (*probability / divisor).max(1);
            }
        }
    }

    let sum = probabilities.iter().sum::<i32>().max(1) as usize;
    let mut acc_log = (sum.ilog2() as u8 + 1).max(5).min(max_log);
    let target = 1usize << acc_log;
    if sum < target {
        let diff = target - sum;
        if let Some(max_probability) = probabilities.iter_mut().max() {
            *max_probability += diff as i32;
        }
    } else if sum > target {
        let mut diff = sum - target;
        while diff > 0 {
            let Some(min_probability) = probabilities.iter_mut().filter(|prob| **prob > 1).min()
            else {
                break;
            };
            let decrease = ((*min_probability as usize) - 1).min(diff);
            *min_probability -= decrease as i32;
            diff -= decrease;
        }
        while probabilities.iter().sum::<i32>() as usize > target {
            if let Some(max_probability) = probabilities.iter_mut().filter(|prob| **prob > 1).max()
            {
                *max_probability -= 1;
            } else {
                break;
            }
        }
    }

    if avoid_0_numbit {
        if let Some((max_index, max_probability)) = probabilities
            .iter()
            .copied()
            .enumerate()
            .max_by_key(|(_, probability)| *probability)
        {
            let limit = 1i32 << (acc_log.saturating_sub(1));
            if max_probability > limit {
                let redistribute = max_probability - limit;
                probabilities[max_index] -= redistribute;
                if let Some((second_index, _)) = probabilities
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(index, _)| *index != max_index)
                    .max_by_key(|(_, probability)| *probability)
                {
                    probabilities[second_index] += redistribute;
                } else {
                    probabilities[max_index] += redistribute;
                }
            }
        }
    }

    acc_log = acc_log.max(5);
    FseHeader {
        probabilities,
        acc_log,
    }
}

fn write_fse_table(output: &mut Vec<u8>, header: &FseHeader) {
    let mut writer = HeaderBitWriter::default();
    writer.write_bits((header.acc_log - 5) as u64, 4);

    let mut probability_counter = 0usize;
    let probability_sum = 1usize << header.acc_log;
    let mut symbol_idx = 0usize;

    while probability_counter < probability_sum && symbol_idx < header.probabilities.len() {
        let max_remaining_value = probability_sum - probability_counter + 1;
        let bits_to_write = max_remaining_value.ilog2() as usize + 1;
        let low_threshold = ((1usize << bits_to_write) - 1).saturating_sub(max_remaining_value);
        let mask = (1usize << (bits_to_write - 1)) - 1;

        let probability = header.probabilities[symbol_idx];
        symbol_idx += 1;
        let value = (probability + 1) as usize;
        if value < low_threshold {
            writer.write_bits(value as u64, bits_to_write - 1);
        } else if value > mask {
            writer.write_bits((value + low_threshold) as u64, bits_to_write);
        } else {
            writer.write_bits(value as u64, bits_to_write);
        }

        if probability > 0 {
            probability_counter += probability as usize;
        } else if probability == -1 {
            probability_counter += 1;
        } else {
            let mut zeros = 0u8;
            while symbol_idx < header.probabilities.len() && header.probabilities[symbol_idx] == 0 {
                zeros += 1;
                symbol_idx += 1;
                if zeros == 3 {
                    writer.write_bits(3, 2);
                    zeros = 0;
                }
            }
            writer.write_bits(zeros as u64, 2);
        }
    }

    output.extend(writer.finish());
}

fn encode_literal_length_symbol(len: u32) -> u8 {
    match len {
        0..=15 => len as u8,
        16..=17 => 16,
        18..=19 => 17,
        20..=21 => 18,
        22..=23 => 19,
        24..=27 => 20,
        28..=31 => 21,
        32..=39 => 22,
        40..=47 => 23,
        48..=63 => 24,
        64..=127 => 25,
        128..=255 => 26,
        256..=511 => 27,
        512..=1023 => 28,
        1024..=2047 => 29,
        2048..=4095 => 30,
        4096..=8191 => 31,
        8192..=16383 => 32,
        16384..=32767 => 33,
        32768..=65535 => 34,
        _ => 35,
    }
}

fn encode_match_length_symbol(len: u32) -> u8 {
    match len {
        0..=34 => len.saturating_sub(3) as u8,
        35..=36 => 32,
        37..=38 => 33,
        39..=40 => 34,
        41..=42 => 35,
        43..=46 => 36,
        47..=50 => 37,
        51..=58 => 38,
        59..=66 => 39,
        67..=82 => 40,
        83..=98 => 41,
        99..=130 => 42,
        131..=258 => 43,
        259..=514 => 44,
        515..=1026 => 45,
        1027..=2050 => 46,
        2051..=4098 => 47,
        4099..=8194 => 48,
        8195..=16386 => 49,
        16387..=32770 => 50,
        32771..=65538 => 51,
        _ => 52,
    }
}

fn encode_offset_with_history(
    actual_offset: u32,
    lit_len: u32,
    offset_history: &mut [u32; 3],
) -> u32 {
    let encoded = if lit_len > 0 {
        if actual_offset == offset_history[0] {
            1
        } else if actual_offset == offset_history[1] {
            2
        } else if actual_offset == offset_history[2] {
            3
        } else {
            actual_offset + 3
        }
    } else if actual_offset == offset_history[1] {
        1
    } else if actual_offset == offset_history[2] {
        2
    } else if offset_history[0] > 1 && actual_offset == offset_history[0] - 1 {
        3
    } else {
        actual_offset + 3
    };

    match (lit_len > 0, encoded) {
        (true, 1) => {}
        (true, 2) => {
            offset_history[1] = offset_history[0];
            offset_history[0] = actual_offset;
        }
        _ => {
            offset_history[2] = offset_history[1];
            offset_history[1] = offset_history[0];
            offset_history[0] = actual_offset;
        }
    }

    encoded
}

fn collect_entropy_stats(samples: &[&[u8]], dict_content: &[u8]) -> EntropyStats {
    let mut stats = EntropyStats::new();
    let block_size = 128 * 1024;
    let history_limit = dict_content.len().max(block_size);
    let mut config = OxiarcLevelConfig::for_level(4);
    config.target_block_size = block_size;

    for sample in samples {
        let mut history = dict_content.to_vec();
        let mut offset_history = DEFAULT_REPCODES;
        for chunk in sample.chunks(block_size) {
            let mut finder = OxiarcMatchFinder::new(&config);
            let sequences = finder
                .find_sequences(chunk, history.as_slice())
                .unwrap_or_default();
            for sequence in sequences {
                if sequence.match_length == 0 {
                    for byte in &sequence.literals {
                        stats.literal_counts[*byte as usize] += 1;
                    }
                    continue;
                }

                for byte in &sequence.literals {
                    stats.literal_counts[*byte as usize] += 1;
                }
                let ll_symbol =
                    encode_literal_length_symbol(sequence.literals.len() as u32) as usize;
                let ml_symbol = encode_match_length_symbol(sequence.match_length as u32) as usize;
                let encoded_offset = encode_offset_with_history(
                    sequence.offset as u32,
                    sequence.literals.len() as u32,
                    &mut offset_history,
                );
                let of_symbol = encoded_offset.ilog2().min(MAX_OFFCODE as u32) as usize;
                stats.lit_length_counts[ll_symbol] += 1;
                stats.match_length_counts[ml_symbol] += 1;
                stats.offcode_counts[of_symbol] += 1;
            }

            history.extend_from_slice(chunk);
            if history.len() > history_limit {
                let trim = history.len() - history_limit;
                history.drain(..trim);
            }
        }
    }

    if stats.literal_counts.iter().all(|count| *count == 0) {
        stats.literal_counts[b' ' as usize] = 1;
        stats.literal_counts[b'e' as usize] = 1;
    }

    stats
}

fn fallback_huffman_table() -> &'static [u8] {
    let size =
        parse_huf_table_size(&FALLBACK_ENTROPY_TABLES).unwrap_or(FALLBACK_ENTROPY_TABLES.len());
    &FALLBACK_ENTROPY_TABLES[..size]
}

fn build_entropy_tables(samples: &[&[u8]], dict_content: &[u8]) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let stats = collect_entropy_stats(samples, dict_content);
    let mut output = Vec::new();
    match build_huffman_table(&stats.literal_counts) {
        Ok(table) => output.extend_from_slice(&table),
        Err(_) => output.extend_from_slice(fallback_huffman_table()),
    }
    let off_table = build_fse_header(&stats.offcode_counts, 8, true);
    write_fse_table(&mut output, &off_table);
    let match_table = build_fse_header(&stats.match_length_counts, 9, true);
    write_fse_table(&mut output, &match_table);
    let lit_table = build_fse_header(&stats.lit_length_counts, 9, true);
    write_fse_table(&mut output, &lit_table);
    Ok(output)
}

fn normalized_dict_id(content: &[u8], requested: u32) -> u32 {
    if requested != 0 {
        return requested;
    }
    ((crate::ffi::compress::xxh64(content) % (((1u64 << 31) - 32_768) as u64)) + 32_768) as u32
}

fn assemble_training_content(
    samples: &[&[u8]],
    target_size: usize,
    preferred_segment_size: usize,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    if target_size < MIN_DICT_CONTENT_SIZE {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    let segment_size = preferred_segment_size.clamp(8, target_size.max(8));
    let ranked = divsufsort::rank_segments(samples, segment_size);
    let mut content = Vec::with_capacity(target_size);

    for segment in ranked {
        if content.len() >= target_size {
            break;
        }
        let remaining = target_size - content.len();
        let take = remaining.min(segment.bytes.len());
        content.extend_from_slice(&segment.bytes[..take]);
    }

    if content.len() < target_size {
        for sample in samples.iter().rev() {
            if content.len() >= target_size {
                break;
            }
            if sample.is_empty() {
                continue;
            }
            let remaining = target_size - content.len();
            let start = sample.len().saturating_sub(remaining);
            content.extend_from_slice(&sample[start..]);
        }
    }

    if content.len() < target_size {
        let mut seed = content.clone();
        while content.len() < target_size && !seed.is_empty() {
            let remaining = target_size - content.len();
            let take = remaining.min(seed.len());
            content.extend_from_slice(&seed[..take]);
            seed.rotate_left(1);
        }
    }

    if content.len() < MIN_DICT_CONTENT_SIZE {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed);
    }

    content.truncate(target_size);
    Ok(content)
}

fn validate_training_samples(samples: &[&[u8]]) -> Result<(), ZSTD_ErrorCode> {
    if samples.len() < MIN_TRAINING_SAMPLES {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }
    Ok(())
}

fn write_formatted_dictionary(
    dst: &mut [u8],
    content: &[u8],
    dict_id: u32,
    entropy_tables: &[u8],
) -> Result<usize, ZSTD_ErrorCode> {
    let header_size = 8usize
        .checked_add(entropy_tables.len())
        .and_then(|size| size.checked_add(size_of::<[u32; 3]>()))
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall)?;
    let total_size = header_size
        .checked_add(content.len())
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall)?;
    if total_size > dst.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    dst[..4].copy_from_slice(&DICTIONARY_MAGIC);
    dst[4..8].copy_from_slice(&dict_id.to_le_bytes());
    dst[8..8 + entropy_tables.len()].copy_from_slice(entropy_tables);

    let repcodes_offset = 8 + entropy_tables.len();
    let max_offset =
        u32::try_from(content.len()).map_err(|_| ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall)?;
    for (index, base) in DEFAULT_REPCODES.iter().copied().enumerate() {
        let repcode = base.min(max_offset).max(1);
        let start = repcodes_offset + index * 4;
        dst[start..start + 4].copy_from_slice(&repcode.to_le_bytes());
    }

    dst[header_size..total_size].copy_from_slice(content);
    Ok(total_size)
}

fn build_formatted_dictionary(
    dst: &mut [u8],
    content: &[u8],
    dict_id_source: &[u8],
    samples: &[&[u8]],
    requested_dict_id: u32,
) -> Result<usize, ZSTD_ErrorCode> {
    let entropy_tables = build_entropy_tables(samples, content)?;
    let dict_id = normalized_dict_id(dict_id_source, requested_dict_id);
    write_formatted_dictionary(dst, content, dict_id, &entropy_tables)
}

pub(crate) fn synthesize_formatted_dictionary(
    content: &[u8],
    dict_id_source: &[u8],
    requested_dict_id: u32,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let mut dst = vec![
        0u8;
        TRAINED_DICT_HEADER_SIZE
            .saturating_add(content.len())
            .saturating_add(512)
    ];
    let samples = [content];
    let size = build_formatted_dictionary(
        dst.as_mut_slice(),
        content,
        dict_id_source,
        &samples,
        requested_dict_id,
    )?;
    dst.truncate(size);
    Ok(dst)
}

fn trained_dictionary_size(
    dict_buffer: *mut c_void,
    dict_buffer_capacity: usize,
    samples_buffer: *const c_void,
    samples_sizes: *const usize,
    nb_samples: c_uint,
    preferred_segment_size: usize,
    params: ZDICT_params_t,
    shrink_dict: bool,
) -> usize {
    let result = (|| {
        let dst = dst_slice_mut(dict_buffer, dict_buffer_capacity)?;
        let samples = parse_samples(samples_buffer, samples_sizes, nb_samples)?;
        validate_training_samples(&samples)?;
        let mut target_size = dst
            .len()
            .saturating_sub(TRAINED_DICT_HEADER_SIZE)
            .max(MIN_DICT_CONTENT_SIZE);
        if shrink_dict && target_size > 512 {
            target_size = (target_size * 3 / 4).max(256);
        }
        // Keep dictionary training deterministic and sensitive to the cover/fastcover
        // segment sizing inputs. The oxiarc raw trainer ignores those knobs, which can
        // make a dictID-only change perturb the optimized dictionary body.
        let content = assemble_training_content(&samples, target_size, preferred_segment_size)?;
        build_formatted_dictionary(dst, &content, &content, &samples, params.dictID)
    })();

    match result {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

pub(crate) fn train_dictionary(
    dict_buffer: *mut c_void,
    dict_buffer_capacity: usize,
    samples_buffer: *const c_void,
    samples_sizes: *const usize,
    nb_samples: c_uint,
    preferred_segment_size: usize,
    params: ZDICT_params_t,
    shrink_dict: bool,
) -> usize {
    trained_dictionary_size(
        dict_buffer,
        dict_buffer_capacity,
        samples_buffer,
        samples_sizes,
        nb_samples,
        preferred_segment_size,
        params,
        shrink_dict,
    )
}

pub(crate) fn evaluate_dictionary_score(
    dictionary: &[u8],
    samples: &[&[u8]],
    compression_level: i32,
) -> Result<usize, ZSTD_ErrorCode> {
    let raw_dictionary = formatted_dictionary_content(dictionary).unwrap_or(dictionary);
    let mut total = 0usize;
    for sample in samples {
        let mut encoder = oxiarc_zstd::ZstdEncoder::new();
        encoder
            .set_level(compression_level.max(1))
            .set_content_size(false)
            .set_checksum(false);
        encoder.set_dictionary(raw_dictionary);
        let encoded = encoder
            .compress(sample)
            .map_err(|_| ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
        total = total
            .checked_add(encoded.len())
            .ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    }
    Ok(total)
}

fn finalized_dictionary_size(
    dst_dict_buffer: *mut c_void,
    max_dict_size: usize,
    dict_content: *const c_void,
    dict_content_size: usize,
    samples_buffer: *const c_void,
    samples_sizes: *const usize,
    nb_samples: c_uint,
    parameters: ZDICT_params_t,
) -> usize {
    let result = (|| {
        if max_dict_size < TRAINED_DICT_HEADER_SIZE + MIN_DICT_CONTENT_SIZE {
            return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
        }
        let dst = dst_slice_mut(dst_dict_buffer, max_dict_size)?;
        let dict_content = src_slice(dict_content, dict_content_size)?;
        let samples = parse_samples(samples_buffer, samples_sizes, nb_samples)?;
        if dict_content.len() < MIN_DICT_CONTENT_SIZE {
            return Err(ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed);
        }

        let max_content_size = max_dict_size - TRAINED_DICT_HEADER_SIZE;
        let start = dict_content.len().saturating_sub(max_content_size);
        let content = dict_content[start..].to_vec();
        build_formatted_dictionary(dst, &content, dict_content, &samples, parameters.dictID)
    })();

    match result {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZDICT_addEntropyTablesFromBuffer(
    dictBuffer: *mut c_void,
    dictContentSize: usize,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
) -> usize {
    if let Err(code) = parse_samples(samplesBuffer, samplesSizes, nbSamples) {
        return error_result(code);
    }
    let dict_content = match src_slice(dictBuffer.cast_const(), dictContentSize) {
        Ok(content) => content,
        Err(code) => return error_result(code),
    };
    let params = ZDICT_params_t::default();
    finalized_dictionary_size(
        dictBuffer,
        dictBufferCapacity,
        dict_content.as_ptr().cast(),
        dict_content.len(),
        samplesBuffer,
        samplesSizes,
        nbSamples,
        params,
    )
}

#[no_mangle]
pub extern "C" fn ZDICT_finalizeDictionary(
    dstDictBuffer: *mut c_void,
    maxDictSize: usize,
    dictContent: *const c_void,
    dictContentSize: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: ZDICT_params_t,
) -> usize {
    finalized_dictionary_size(
        dstDictBuffer,
        maxDictSize,
        dictContent,
        dictContentSize,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        parameters,
    )
}

#[no_mangle]
pub extern "C" fn ZDICT_trainFromBuffer_legacy(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: ZDICT_legacy_params_t,
) -> usize {
    let segment_size = 32usize.saturating_add(parameters.selectivityLevel as usize * 8);
    train_dictionary(
        dictBuffer,
        dictBufferCapacity,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        segment_size,
        parameters.zParams,
        false,
    )
}

#[no_mangle]
pub extern "C" fn ZDICT_getDictHeaderSize(dictBuffer: *const c_void, dictSize: usize) -> usize {
    match src_slice(dictBuffer, dictSize).and_then(dictionary_header_size) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZDICT_getDictID(dictBuffer: *const c_void, dictSize: usize) -> c_uint {
    match src_slice(dictBuffer, dictSize) {
        Ok(bytes) if bytes.len() >= 8 && bytes[..4] == DICTIONARY_MAGIC => {
            u32::from_le_bytes(bytes[4..8].try_into().expect("slice length checked"))
        }
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn ZDICT_trainFromBuffer(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
) -> usize {
    train_dictionary(
        dictBuffer,
        dictBufferCapacity,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        8,
        ZDICT_params_t::default(),
        false,
    )
}

#[no_mangle]
pub extern "C" fn ZDICT_isError(errorCode: usize) -> c_uint {
    is_error_result(errorCode) as c_uint
}

#[no_mangle]
pub extern "C" fn ZDICT_getErrorName(errorCode: usize) -> *const c_char {
    crate::common::error::ZSTD_getErrorName(errorCode)
}
