use crate::{
    common::MAX_BLOCK_SIZE,
    encoding::{
        Matcher, block_header::BlockHeader, blocks::compress_block, frame_compressor::CompressState,
    },
};
use alloc::vec::Vec;

/// Compresses a single block at [`crate::encoding::CompressionLevel::Fastest`].
///
/// # Parameters
/// - `state`: [`CompressState`] so the compressor can refer to data before
///   the start of this block
/// - `last_block`: Whether or not this block is going to be the last block in the frame
///   (needed because this info is written into the block header)
/// - `uncompressed_data`: A block's worth of uncompressed data, taken from the
///   larger input
/// - `output`: As `uncompressed_data` is compressed, it's appended to `output`.
#[inline]
pub fn compress_fastest<M: Matcher>(
    state: &mut CompressState<M>,
    last_block: bool,
    uncompressed_data: Vec<u8>,
    output: &mut Vec<u8>,
) {
    let block_size = uncompressed_data.len() as u32;
    // First check to see if run length encoding can be used for the entire block
    if uncompressed_data.iter().all(|x| uncompressed_data[0].eq(x)) {
        let rle_byte = uncompressed_data[0];
        state.matcher.commit_space(uncompressed_data);
        state.matcher.skip_matching();
        let header = BlockHeader {
            last_block,
            block_type: crate::blocks::block::BlockType::RLE,
            block_size,
        };
        // Write the header, then the block
        header.serialize(output);
        output.push(rle_byte);
    } else {
        let previous_huff_table = state.last_huff_table.clone();
        let previous_fse_tables = state.fse_tables.clone();
        let previous_offset_hist = state.offset_hist;
        let previous_matcher_offsets = state.matcher.repeat_offsets();
        // Compress as a standard compressed block
        let mut compressed = Vec::new();
        state.matcher.commit_space(uncompressed_data);
        let raw_block = state.matcher.get_last_space().to_vec();
        compress_block(state, &mut compressed);
        // Raw blocks are always cheaper when the compressed payload doesn't
        // beat the original byte count, and they are also required once the
        // compressed payload exceeds the maximum encodable block size.
        if compressed.len() >= raw_block.len() || compressed.len() >= MAX_BLOCK_SIZE as usize {
            state.last_huff_table = previous_huff_table;
            state.fse_tables = previous_fse_tables;
            state.offset_hist = previous_offset_hist;
            if let Some(offsets) = previous_matcher_offsets {
                state.matcher.restore_repeat_offsets(offsets);
            }
            let header = BlockHeader {
                last_block,
                block_type: crate::blocks::block::BlockType::Raw,
                block_size,
            };
            // Write the header, then the block
            header.serialize(output);
            output.extend_from_slice(&raw_block);
        } else {
            let header = BlockHeader {
                last_block,
                block_type: crate::blocks::block::BlockType::Compressed,
                block_size: compressed.len() as u32,
            };
            // Write the header, then the block
            header.serialize(output);
            output.extend(compressed);
        }
    }
}
