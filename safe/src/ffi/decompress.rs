use crate::{
    common::{
        alloc,
        error::{decode_error, is_error_result},
    },
    decompress::{
        block::{parse_block_header, BlockHeader, BlockType, BLOCK_HEADER_SIZE, BLOCK_SIZE_MAX},
        frame::{self, DictionaryRef},
    },
    ffi::types::{
        ZSTD_DCtx, ZSTD_DDict, ZSTD_ErrorCode, ZSTD_dParameter, ZSTD_dictContentType_e,
        ZSTD_format_e, ZSTD_inBuffer, ZSTD_outBuffer,
    },
};
use core::ffi::c_void;

fn validate_formatted_dictionary(bytes: &[u8]) -> Result<(), ZSTD_ErrorCode> {
    type CreateDDict = unsafe extern "C" fn(*const c_void, usize) -> *mut ZSTD_DDict;
    type FreeDDict = unsafe extern "C" fn(*mut ZSTD_DDict) -> usize;

    let Some(create_ddict) =
        crate::ffi::compress::load_upstream!("ZSTD_createDDict", CreateDDict)
    else {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    let Some(free_ddict) = crate::ffi::compress::load_upstream!("ZSTD_freeDDict", FreeDDict)
    else {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };

    let ddict = unsafe { create_ddict(bytes.as_ptr().cast(), bytes.len()) };
    if ddict.is_null() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
    }
    unsafe {
        free_ddict(ddict);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DictionaryUse {
    Once,
    Indefinitely,
}

#[derive(Clone, Debug)]
enum DictionarySelection {
    None,
    Referenced(*const DecoderDictionary),
    Owned {
        raw: Vec<u8>,
        formatted: bool,
        dict_id: u32,
        use_mode: DictionaryUse,
    },
}

impl Default for DictionarySelection {
    fn default() -> Self {
        Self::None
    }
}

impl DictionarySelection {
    fn clear(&mut self) {
        *self = Self::None;
    }

    fn resolve<'a>(&'a self) -> Result<DictionaryRef<'a>, ZSTD_ErrorCode> {
        match self {
            DictionarySelection::None => Ok(DictionaryRef::None),
            DictionarySelection::Referenced(ptr) => {
                let ddict = ddict_ref(*ptr).ok_or(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong)?;
                Ok(ddict.as_dictionary_ref())
            }
            DictionarySelection::Owned {
                raw,
                formatted,
                dict_id,
                ..
            } => Ok(if *formatted {
                let _ = dict_id;
                DictionaryRef::Formatted(raw)
            } else {
                DictionaryRef::Raw(raw)
            }),
        }
    }

    fn consume_once(&mut self) {
        if matches!(
            self,
            DictionarySelection::Owned {
                use_mode: DictionaryUse::Once,
                ..
            }
        ) {
            self.clear();
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DecoderDictionary {
    pub raw: Vec<u8>,
    pub dict_id: u32,
    pub formatted: bool,
}

impl DecoderDictionary {
    #[allow(dead_code)]
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, ZSTD_ErrorCode> {
        Self::from_bytes_with_content_type(bytes, ZSTD_dictContentType_e::ZSTD_dct_auto)
    }

    pub(crate) fn from_bytes_with_content_type(
        bytes: &[u8],
        dict_content_type: ZSTD_dictContentType_e,
    ) -> Result<Self, ZSTD_ErrorCode> {
        let formatted = match dict_content_type {
            ZSTD_dictContentType_e::ZSTD_dct_auto => {
                crate::decompress::fse::validate_dictionary_kind(bytes)?;
                let formatted = crate::decompress::huf::is_formatted_dictionary(bytes);
                if formatted {
                    validate_formatted_dictionary(bytes)?;
                }
                formatted
            }
            ZSTD_dictContentType_e::ZSTD_dct_rawContent => false,
            ZSTD_dictContentType_e::ZSTD_dct_fullDict => {
                validate_formatted_dictionary(bytes)?;
                true
            }
        };
        Ok(Self {
            raw: bytes.to_vec(),
            dict_id: if formatted {
                crate::decompress::fse::formatted_dict_id(bytes)
            } else {
                0
            },
            formatted,
        })
    }

    pub(crate) fn as_dictionary_ref(&self) -> DictionaryRef<'_> {
        if self.formatted {
            let _ = self.dict_id;
            DictionaryRef::Formatted(&self.raw)
        } else {
            DictionaryRef::Raw(&self.raw)
        }
    }

    pub(crate) fn heap_size(&self) -> usize {
        alloc::heap_bytes(self.raw.len())
    }
}

#[derive(Clone, Debug, Default)]
struct StreamState {
    compressed: Vec<u8>,
    decoded: Vec<u8>,
    output_pos: usize,
    deferred_input_advance: usize,
}

impl StreamState {
    fn reset(&mut self) {
        self.compressed.clear();
        self.decoded.clear();
        self.output_pos = 0;
        self.deferred_input_advance = 0;
    }

    fn is_busy(&self) -> bool {
        !self.compressed.is_empty()
            || self.output_pos < self.decoded.len()
            || self.deferred_input_advance != 0
    }

    fn size_of(&self) -> usize {
        alloc::heap_bytes(self.compressed.len() + self.decoded.len())
    }
}

#[derive(Debug)]
struct UpstreamBufferlessSession {
    ptr: *mut ZSTD_DCtx,
}

impl UpstreamBufferlessSession {
    fn new(dctx: &DecoderContext) -> Result<Self, ZSTD_ErrorCode> {
        Self::new_inner(dctx, false)
    }

    fn new_stream(dctx: &DecoderContext) -> Result<Self, ZSTD_ErrorCode> {
        Self::new_inner(dctx, true)
    }

    fn new_inner(dctx: &DecoderContext, stream_mode: bool) -> Result<Self, ZSTD_ErrorCode> {
        type CreateDCtx = unsafe extern "C" fn() -> *mut ZSTD_DCtx;
        type SetFormat = unsafe extern "C" fn(*mut ZSTD_DCtx, ZSTD_format_e) -> usize;
        type SetMaxWindowSize = unsafe extern "C" fn(*mut ZSTD_DCtx, usize) -> usize;
        type DecompressBegin = unsafe extern "C" fn(*mut ZSTD_DCtx) -> usize;
        type DecompressBeginUsingDict =
            unsafe extern "C" fn(*mut ZSTD_DCtx, *const c_void, usize) -> usize;
        type InitDStream = unsafe extern "C" fn(*mut ZSTD_DCtx) -> usize;
        type InitDStreamUsingDict =
            unsafe extern "C" fn(*mut ZSTD_DCtx, *const c_void, usize) -> usize;

        let Some(create_dctx) = crate::ffi::compress::load_upstream!("ZSTD_createDCtx", CreateDCtx)
        else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };
        let Some(set_format) =
            crate::ffi::compress::load_upstream!("ZSTD_DCtx_setFormat", SetFormat)
        else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };
        let Some(set_max_window_size) = crate::ffi::compress::load_upstream!(
            "ZSTD_DCtx_setMaxWindowSize",
            SetMaxWindowSize
        ) else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };
        let Some(decompress_begin) =
            crate::ffi::compress::load_upstream!("ZSTD_decompressBegin", DecompressBegin)
        else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };
        let Some(decompress_begin_using_dict) = crate::ffi::compress::load_upstream!(
            "ZSTD_decompressBegin_usingDict",
            DecompressBeginUsingDict
        ) else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };
        let Some(init_dstream) = crate::ffi::compress::load_upstream!("ZSTD_initDStream", InitDStream)
        else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };
        let Some(init_dstream_using_dict) = crate::ffi::compress::load_upstream!(
            "ZSTD_initDStream_usingDict",
            InitDStreamUsingDict
        ) else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };

        let ptr = unsafe { create_dctx() };
        if ptr.is_null() {
            return Err(ZSTD_ErrorCode::ZSTD_error_memory_allocation);
        }

        let session = Self { ptr };
        let result = (|| {
            let setup = unsafe { set_format(session.ptr, dctx.format) };
            if is_error_result(setup) {
                return Err(decode_error(setup));
            }
            let setup = unsafe { set_max_window_size(session.ptr, dctx.max_window_size) };
            if is_error_result(setup) {
                return Err(decode_error(setup));
            }

            let init = match (stream_mode, dctx.resolved_dict()?) {
                (false, DictionaryRef::None) => unsafe { decompress_begin(session.ptr) },
                (false, DictionaryRef::Raw(bytes) | DictionaryRef::Formatted(bytes)) => unsafe {
                    decompress_begin_using_dict(session.ptr, bytes.as_ptr().cast(), bytes.len())
                },
                (true, DictionaryRef::None) => unsafe { init_dstream(session.ptr) },
                (true, DictionaryRef::Raw(bytes) | DictionaryRef::Formatted(bytes)) => unsafe {
                    init_dstream_using_dict(session.ptr, bytes.as_ptr().cast(), bytes.len())
                },
            };
            if is_error_result(init) {
                return Err(decode_error(init));
            }

            Ok(())
        })();

        if let Err(code) = result {
            drop(session);
            return Err(code);
        }

        Ok(session)
    }

    fn continue_decode(
        &mut self,
        dst: *mut c_void,
        dst_capacity: usize,
        src: &[u8],
    ) -> Result<usize, ZSTD_ErrorCode> {
        type DecompressContinue =
            unsafe extern "C" fn(*mut ZSTD_DCtx, *mut c_void, usize, *const c_void, usize) -> usize;

        let Some(decompress_continue) = crate::ffi::compress::load_upstream!(
            "ZSTD_decompressContinue",
            DecompressContinue
        ) else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };

        let wrote = unsafe {
            decompress_continue(
                self.ptr,
                dst,
                dst_capacity,
                src.as_ptr().cast(),
                src.len(),
            )
        };
        if is_error_result(wrote) {
            Err(decode_error(wrote))
        } else {
            Ok(wrote)
        }
    }

    fn next_src_size_to_decompress(&mut self) -> Result<usize, ZSTD_ErrorCode> {
        type NextSrcSizeToDecompress = unsafe extern "C" fn(*mut ZSTD_DCtx) -> usize;

        let Some(next_src_size_to_decompress) = crate::ffi::compress::load_upstream!(
            "ZSTD_nextSrcSizeToDecompress",
            NextSrcSizeToDecompress
        ) else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };

        Ok(unsafe { next_src_size_to_decompress(self.ptr) })
    }

    fn decompress_block(
        &mut self,
        dst: *mut c_void,
        dst_capacity: usize,
        src: &[u8],
    ) -> Result<usize, ZSTD_ErrorCode> {
        type DecompressBlock =
            unsafe extern "C" fn(*mut ZSTD_DCtx, *mut c_void, usize, *const c_void, usize) -> usize;

        let Some(decompress_block) =
            crate::ffi::compress::load_upstream!("ZSTD_decompressBlock", DecompressBlock)
        else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };

        let wrote = unsafe {
            decompress_block(
                self.ptr,
                dst,
                dst_capacity,
                src.as_ptr().cast(),
                src.len(),
            )
        };
        if is_error_result(wrote) {
            Err(decode_error(wrote))
        } else {
            Ok(wrote)
        }
    }

    fn decompress_stream(
        &mut self,
        output: &mut ZSTD_outBuffer,
        input: &mut ZSTD_inBuffer,
    ) -> Result<usize, ZSTD_ErrorCode> {
        type DecompressStream =
            unsafe extern "C" fn(*mut ZSTD_DCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;

        let Some(decompress_stream) =
            crate::ffi::compress::load_upstream!("ZSTD_decompressStream", DecompressStream)
        else {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        };

        let wrote = unsafe { decompress_stream(self.ptr, output, input) };
        if is_error_result(wrote) {
            Err(decode_error(wrote))
        } else {
            Ok(wrote)
        }
    }
}

impl Drop for UpstreamBufferlessSession {
    fn drop(&mut self) {
        type FreeDCtx = unsafe extern "C" fn(*mut ZSTD_DCtx) -> usize;

        if self.ptr.is_null() {
            return;
        }

        let Some(free_dctx) = crate::ffi::compress::load_upstream!("ZSTD_freeDCtx", FreeDCtx)
        else {
            return;
        };

        unsafe {
            free_dctx(self.ptr);
        }
        self.ptr = core::ptr::null_mut();
    }
}

fn stage_decoded_output(dctx: &mut DecoderContext, decoded: &[u8]) {
    dctx.stream.decoded.clear();
    dctx.stream.decoded.extend_from_slice(decoded);
    dctx.stream.output_pos = 0;
}

#[allow(dead_code)]
fn drain_staged_output(
    dctx: &mut DecoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
) -> Result<usize, ZSTD_ErrorCode> {
    let remaining = &dctx.stream.decoded[dctx.stream.output_pos..];
    if remaining.is_empty() {
        return Ok(0);
    }
    if dst_capacity == 0 {
        return Err(ZSTD_ErrorCode::ZSTD_error_noForwardProgress_destFull);
    }
    let to_write = remaining.len().min(dst_capacity);
    // SAFETY: The caller provides `dst_capacity` writable bytes at `dst`.
    unsafe {
        core::ptr::copy_nonoverlapping(remaining.as_ptr(), dst.cast::<u8>(), to_write);
    }
    dctx.stream.output_pos += to_write;
    if dctx.stream.output_pos == dctx.stream.decoded.len() {
        dctx.stream.decoded.clear();
        dctx.stream.output_pos = 0;
    }
    Ok(to_write)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BufferlessStage {
    Idle,
    NeedStart,
    NeedHeaderRemainder(usize),
    NeedSkippableHeaderRemainder(usize),
    NeedBlockHeader,
    NeedBlockBody(BlockHeader),
    NeedChecksum(usize),
    NeedSkippablePayload(usize),
    Finished,
}

#[derive(Debug)]
struct BufferlessState {
    stage: BufferlessStage,
    frame_bytes: Vec<u8>,
    header: Option<crate::ffi::types::ZSTD_frameHeader>,
    upstream: Option<UpstreamBufferlessSession>,
}

impl Default for BufferlessState {
    fn default() -> Self {
        Self {
            stage: BufferlessStage::Idle,
            frame_bytes: Vec::new(),
            header: None,
            upstream: None,
        }
    }
}

impl Clone for BufferlessState {
    fn clone(&self) -> Self {
        Self {
            stage: self.stage.clone(),
            frame_bytes: self.frame_bytes.clone(),
            header: self.header,
            upstream: None,
        }
    }
}

impl BufferlessState {
    fn begin(&mut self) {
        self.stage = BufferlessStage::NeedStart;
        self.frame_bytes.clear();
        self.header = None;
        self.upstream = None;
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn is_busy(&self) -> bool {
        !matches!(self.stage, BufferlessStage::Idle | BufferlessStage::Finished)
    }

    fn next_src_size(&self, format: ZSTD_format_e) -> usize {
        match self.stage {
            BufferlessStage::Idle => 0,
            BufferlessStage::NeedStart => frame::starting_input_length(format),
            BufferlessStage::NeedHeaderRemainder(size) => size,
            BufferlessStage::NeedSkippableHeaderRemainder(size) => size,
            BufferlessStage::NeedBlockHeader => BLOCK_HEADER_SIZE,
            BufferlessStage::NeedBlockBody(header) => {
                if header.block_type == BlockType::Rle {
                    1
                } else {
                    header.content_size
                }
            }
            BufferlessStage::NeedChecksum(size) => size,
            BufferlessStage::NeedSkippablePayload(size) => size,
            BufferlessStage::Finished => 0,
        }
    }

    fn next_input_type(
        &self,
    ) -> crate::ffi::types::ZSTD_nextInputType_e {
        use crate::ffi::types::ZSTD_nextInputType_e as Next;

        match self.stage {
            BufferlessStage::NeedStart | BufferlessStage::NeedHeaderRemainder(_) => {
                Next::ZSTDnit_frameHeader
            }
            BufferlessStage::NeedSkippableHeaderRemainder(_)
            | BufferlessStage::NeedSkippablePayload(_) => Next::ZSTDnit_skippableFrame,
            BufferlessStage::NeedBlockHeader => Next::ZSTDnit_blockHeader,
            BufferlessStage::NeedBlockBody(header) => {
                if header.last_block {
                    Next::ZSTDnit_lastBlock
                } else {
                    Next::ZSTDnit_block
                }
            }
            BufferlessStage::NeedChecksum(_) => Next::ZSTDnit_checksum,
            BufferlessStage::Idle | BufferlessStage::Finished => Next::ZSTDnit_frameHeader,
        }
    }

    fn size_of(&self) -> usize {
        alloc::heap_bytes(self.frame_bytes.len())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DecoderContext {
    pub format: ZSTD_format_e,
    pub max_window_size: usize,
    stable_out_buffer: i32,
    force_ignore_checksum: i32,
    ref_multiple_ddicts: i32,
    disable_huffman_assembly: i32,
    dict: DictionarySelection,
    stream: StreamState,
    bufferless: BufferlessState,
}

impl Default for DecoderContext {
    fn default() -> Self {
        Self {
            format: ZSTD_format_e::ZSTD_f_zstd1,
            max_window_size: (1usize << frame::ZSTD_WINDOWLOG_LIMIT_DEFAULT) + 1,
            stable_out_buffer: 0,
            force_ignore_checksum: 0,
            ref_multiple_ddicts: 0,
            disable_huffman_assembly: 0,
            dict: DictionarySelection::None,
            stream: StreamState::default(),
            bufferless: BufferlessState::default(),
        }
    }
}

impl DecoderContext {
    pub(crate) fn sizeof(&self) -> usize {
        alloc::base_size::<Self>()
            + match &self.dict {
                DictionarySelection::None => 0,
                DictionarySelection::Referenced(ptr) => ddict_ref(*ptr).map_or(0, DecoderDictionary::heap_size),
                DictionarySelection::Owned { raw, .. } => alloc::heap_bytes(raw.len()),
            }
            + self.stream.size_of()
            + self.bufferless.size_of()
    }

    pub(crate) fn can_set_parameters(&self) -> bool {
        !self.stream.is_busy() && !self.bufferless.is_busy()
    }

    pub(crate) fn reset_session(&mut self) {
        self.stream.reset();
        self.bufferless.reset();
    }

    pub(crate) fn reset_parameters(&mut self) {
        self.format = ZSTD_format_e::ZSTD_f_zstd1;
        self.max_window_size = (1usize << frame::ZSTD_WINDOWLOG_LIMIT_DEFAULT) + 1;
        self.stable_out_buffer = 0;
        self.force_ignore_checksum = 0;
        self.ref_multiple_ddicts = 0;
        self.disable_huffman_assembly = 0;
        self.dict.clear();
        self.reset_session();
    }

    pub(crate) fn copy_from(&mut self, other: &Self) {
        self.format = other.format;
        self.max_window_size = other.max_window_size;
        self.stable_out_buffer = other.stable_out_buffer;
        self.force_ignore_checksum = other.force_ignore_checksum;
        self.ref_multiple_ddicts = other.ref_multiple_ddicts;
        self.disable_huffman_assembly = other.disable_huffman_assembly;
        self.dict = other.dict.clone();
        self.reset_session();
    }

    pub(crate) fn load_dictionary(
        &mut self,
        bytes: &[u8],
        use_mode: DictionaryUse,
    ) -> Result<(), ZSTD_ErrorCode> {
        self.load_dictionary_with_content_type(
            bytes,
            use_mode,
            ZSTD_dictContentType_e::ZSTD_dct_auto,
        )
    }

    pub(crate) fn load_dictionary_with_content_type(
        &mut self,
        bytes: &[u8],
        use_mode: DictionaryUse,
        dict_content_type: ZSTD_dictContentType_e,
    ) -> Result<(), ZSTD_ErrorCode> {
        if !self.can_set_parameters() {
            return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
        }
        if bytes.is_empty() {
            self.dict.clear();
            return Ok(());
        }
        let ddict = DecoderDictionary::from_bytes_with_content_type(bytes, dict_content_type)?;
        self.dict = DictionarySelection::Owned {
            raw: ddict.raw,
            formatted: ddict.formatted,
            dict_id: ddict.dict_id,
            use_mode,
        };
        Ok(())
    }

    pub(crate) fn ref_ddict(&mut self, ddict: *const DecoderDictionary) -> Result<(), ZSTD_ErrorCode> {
        if !self.can_set_parameters() {
            return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
        }
        self.dict = if ddict.is_null() {
            DictionarySelection::None
        } else {
            DictionarySelection::Referenced(ddict)
        };
        Ok(())
    }

    pub(crate) fn resolved_dict(&self) -> Result<DictionaryRef<'_>, ZSTD_ErrorCode> {
        self.dict.resolve()
    }

    pub(crate) fn clear_once_dict(&mut self) {
        self.dict.consume_once();
    }

    pub(crate) fn set_parameter(
        &mut self,
        param: ZSTD_dParameter,
        mut value: i32,
    ) -> Result<(), ZSTD_ErrorCode> {
        if !self.can_set_parameters() {
            return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
        }

        let (lower, upper) = dparam_bounds(param)
            .ok_or(ZSTD_ErrorCode::ZSTD_error_parameter_unsupported)?;
        if param == ZSTD_dParameter::ZSTD_d_windowLogMax && value == 0 {
            value = frame::ZSTD_WINDOWLOG_LIMIT_DEFAULT as i32;
        }
        if value < lower || value > upper {
            return Err(ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound);
        }

        match param {
            ZSTD_dParameter::ZSTD_d_windowLogMax => {
                self.max_window_size = 1usize << value;
            }
            ZSTD_dParameter::ZSTD_d_experimentalParam1 => {
                self.format = match value {
                    x if x == ZSTD_format_e::ZSTD_f_zstd1 as i32 => ZSTD_format_e::ZSTD_f_zstd1,
                    x if x == ZSTD_format_e::ZSTD_f_zstd1_magicless as i32 => {
                        ZSTD_format_e::ZSTD_f_zstd1_magicless
                    }
                    _ => return Err(ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound),
                };
            }
            ZSTD_dParameter::ZSTD_d_experimentalParam2 => {
                self.stable_out_buffer = value;
            }
            ZSTD_dParameter::ZSTD_d_experimentalParam3 => {
                self.force_ignore_checksum = value;
            }
            ZSTD_dParameter::ZSTD_d_experimentalParam4 => {
                self.ref_multiple_ddicts = value;
            }
            ZSTD_dParameter::ZSTD_d_experimentalParam5 => {
                self.disable_huffman_assembly = value;
            }
        }

        Ok(())
    }

    pub(crate) fn get_parameter(&self, param: ZSTD_dParameter) -> Result<i32, ZSTD_ErrorCode> {
        match param {
            ZSTD_dParameter::ZSTD_d_windowLogMax => Ok(self.max_window_size.ilog2() as i32),
            ZSTD_dParameter::ZSTD_d_experimentalParam1 => Ok(self.format as i32),
            ZSTD_dParameter::ZSTD_d_experimentalParam2 => Ok(self.stable_out_buffer),
            ZSTD_dParameter::ZSTD_d_experimentalParam3 => Ok(self.force_ignore_checksum),
            ZSTD_dParameter::ZSTD_d_experimentalParam4 => Ok(self.ref_multiple_ddicts),
            ZSTD_dParameter::ZSTD_d_experimentalParam5 => Ok(self.disable_huffman_assembly),
        }
    }

    pub(crate) fn set_format(&mut self, format: ZSTD_format_e) -> Result<(), ZSTD_ErrorCode> {
        self.set_parameter(ZSTD_dParameter::ZSTD_d_experimentalParam1, format as i32)
    }

    pub(crate) fn set_max_window_size(&mut self, max_window_size: usize) -> Result<(), ZSTD_ErrorCode> {
        let min = 1usize << frame::ZSTD_WINDOWLOG_ABSOLUTEMIN;
        let max = 1usize << frame::ZSTD_WINDOWLOG_MAX;
        if !self.can_set_parameters() {
            return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
        }
        if max_window_size < min || max_window_size > max {
            return Err(ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound);
        }
        self.max_window_size = max_window_size;
        Ok(())
    }

    pub(crate) fn ref_prefix(&mut self, prefix: &[u8]) -> Result<(), ZSTD_ErrorCode> {
        self.ref_prefix_with_content_type(prefix, ZSTD_dictContentType_e::ZSTD_dct_rawContent)
    }

    pub(crate) fn ref_prefix_with_content_type(
        &mut self,
        prefix: &[u8],
        dict_content_type: ZSTD_dictContentType_e,
    ) -> Result<(), ZSTD_ErrorCode> {
        if !self.can_set_parameters() {
            return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
        }
        if prefix.is_empty() {
            self.dict.clear();
            return Ok(());
        }
        let ddict = DecoderDictionary::from_bytes_with_content_type(prefix, dict_content_type)?;
        self.dict = DictionarySelection::Owned {
            raw: ddict.raw,
            formatted: ddict.formatted,
            dict_id: ddict.dict_id,
            use_mode: DictionaryUse::Once,
        };
        Ok(())
    }
}

pub(crate) fn optional_src_slice<'a>(src: *const c_void, src_size: usize) -> Option<&'a [u8]> {
    if src_size == 0 {
        return Some(&[]);
    }
    if src.is_null() {
        return None;
    }
    // SAFETY: The caller provided a readable buffer of `src_size` bytes.
    Some(unsafe { core::slice::from_raw_parts(src.cast::<u8>(), src_size) })
}

fn dctx_mut<'a>(ptr: *mut ZSTD_DCtx) -> Option<&'a mut DecoderContext> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: All public constructors allocate a `DecoderContext` and cast it to `ZSTD_DCtx`.
    Some(unsafe { &mut *ptr.cast::<DecoderContext>() })
}

fn dctx_ref<'a>(ptr: *const ZSTD_DCtx) -> Option<&'a DecoderContext> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: All public constructors allocate a `DecoderContext` and cast it to `ZSTD_DCtx`.
    Some(unsafe { &*ptr.cast::<DecoderContext>() })
}

fn ddict_ref<'a>(ptr: *const DecoderDictionary) -> Option<&'a DecoderDictionary> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: All public constructors allocate a `DecoderDictionary` and cast it to `ZSTD_DDict`.
    Some(unsafe { &*ptr })
}

pub(crate) fn with_dctx_ref<T>(
    ptr: *const ZSTD_DCtx,
    f: impl FnOnce(&DecoderContext) -> Result<T, ZSTD_ErrorCode>,
) -> Result<T, ZSTD_ErrorCode> {
    let dctx = dctx_ref(ptr).ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    f(dctx)
}

pub(crate) fn with_dctx_mut<T>(
    ptr: *mut ZSTD_DCtx,
    f: impl FnOnce(&mut DecoderContext) -> Result<T, ZSTD_ErrorCode>,
) -> Result<T, ZSTD_ErrorCode> {
    let dctx = dctx_mut(ptr).ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    f(dctx)
}

pub(crate) fn create_dctx() -> *mut ZSTD_DCtx {
    Box::into_raw(Box::new(DecoderContext::default())).cast()
}

pub(crate) fn free_dctx(ptr: *mut ZSTD_DCtx) -> usize {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: `ptr` originated from `create_dctx`.
    unsafe { drop(Box::from_raw(ptr.cast::<DecoderContext>())); }
    0
}

#[allow(dead_code)]
pub(crate) fn create_ddict(dict: &[u8]) -> Result<*mut ZSTD_DDict, ZSTD_ErrorCode> {
    create_ddict_with_content_type(dict, ZSTD_dictContentType_e::ZSTD_dct_auto)
}

pub(crate) fn create_ddict_with_content_type(
    dict: &[u8],
    dict_content_type: ZSTD_dictContentType_e,
) -> Result<*mut ZSTD_DDict, ZSTD_ErrorCode> {
    let ddict = DecoderDictionary::from_bytes_with_content_type(dict, dict_content_type)?;
    Ok(Box::into_raw(Box::new(ddict)).cast())
}

pub(crate) fn free_ddict(ptr: *mut ZSTD_DDict) -> usize {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: `ptr` originated from `create_ddict`.
    unsafe { drop(Box::from_raw(ptr.cast::<DecoderDictionary>())); }
    0
}

pub(crate) fn sizeof_dctx(ptr: *const ZSTD_DCtx) -> usize {
    dctx_ref(ptr).map_or(0, DecoderContext::sizeof)
}

pub(crate) fn sizeof_ddict(ptr: *const ZSTD_DDict) -> usize {
    ddict_ref(ptr.cast()).map_or(0, |ddict| alloc::base_size::<DecoderDictionary>() + ddict.heap_size())
}

pub(crate) fn get_dict_id_from_ddict(ptr: *const ZSTD_DDict) -> u32 {
    ddict_ref(ptr.cast()).map_or(0, |ddict| ddict.dict_id)
}

pub(crate) fn with_ddict_ref<T>(
    ptr: *const ZSTD_DDict,
    f: impl FnOnce(&DecoderDictionary) -> Result<T, ZSTD_ErrorCode>,
) -> Result<T, ZSTD_ErrorCode> {
    let ddict = ddict_ref(ptr.cast()).ok_or(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong)?;
    f(ddict)
}

pub(crate) fn begin_bufferless(dctx: &mut DecoderContext) {
    dctx.bufferless.begin();
}

pub(crate) fn next_src_size_to_decompress(ptr: *mut ZSTD_DCtx) -> usize {
    dctx_mut(ptr)
        .map(|dctx| dctx.bufferless.next_src_size(dctx.format))
        .unwrap_or(0)
}

pub(crate) fn next_input_type(
    ptr: *mut ZSTD_DCtx,
) -> crate::ffi::types::ZSTD_nextInputType_e {
    dctx_mut(ptr)
        .map(|dctx| dctx.bufferless.next_input_type())
        .unwrap_or(crate::ffi::types::ZSTD_nextInputType_e::ZSTDnit_frameHeader)
}

pub(crate) fn bufferless_continue(
    dctx: &mut DecoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    src: &[u8],
    allow_staging: bool,
) -> Result<usize, ZSTD_ErrorCode> {
    if dctx.bufferless.next_src_size(dctx.format) != src.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    match dctx.bufferless.stage.clone() {
        BufferlessStage::Idle => Err(ZSTD_ErrorCode::ZSTD_error_init_missing),
        BufferlessStage::NeedStart => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            if dctx.format != ZSTD_format_e::ZSTD_f_zstd1_magicless
                && matches!(frame::classify_frame(&dctx.bufferless.frame_bytes), Some(frame::FrameKind::Legacy(_)))
            {
                return Err(ZSTD_ErrorCode::ZSTD_error_version_unsupported);
            }
            match frame::parse_frame_header(&dctx.bufferless.frame_bytes, dctx.format)? {
                frame::HeaderProbe::Need(size) => {
                    dctx.bufferless.stage = if matches!(
                        frame::classify_frame(&dctx.bufferless.frame_bytes),
                        Some(frame::FrameKind::Skippable)
                    ) {
                        BufferlessStage::NeedSkippableHeaderRemainder(
                            size - dctx.bufferless.frame_bytes.len(),
                        )
                    } else {
                        BufferlessStage::NeedHeaderRemainder(size - dctx.bufferless.frame_bytes.len())
                    };
                    forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)
                }
                frame::HeaderProbe::Header(header) => {
                    dctx.bufferless.header = Some(header);
                    if header.frameType
                        == crate::ffi::types::ZSTD_frameType_e::ZSTD_skippableFrame
                    {
                        let payload_size = usize::try_from(header.frameContentSize)
                            .map_err(|_| ZSTD_ErrorCode::ZSTD_error_frameParameter_unsupported)?;
                        dctx.bufferless.stage = if payload_size == 0 {
                            dctx.clear_once_dict();
                            BufferlessStage::Finished
                        } else {
                            BufferlessStage::NeedSkippablePayload(payload_size)
                        };
                    } else {
                        dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
                    }
                    forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)
                }
            }
        }
        BufferlessStage::NeedHeaderRemainder(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            match frame::parse_frame_header(&dctx.bufferless.frame_bytes, dctx.format)? {
                frame::HeaderProbe::Need(size) => {
                    dctx.bufferless.stage =
                        BufferlessStage::NeedHeaderRemainder(size - dctx.bufferless.frame_bytes.len());
                    forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)
                }
                frame::HeaderProbe::Header(header) => {
                    dctx.bufferless.header = Some(header);
                    dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
                    forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)
                }
            }
        }
        BufferlessStage::NeedSkippableHeaderRemainder(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            match frame::parse_frame_header(&dctx.bufferless.frame_bytes, dctx.format)? {
                frame::HeaderProbe::Need(size) => {
                    dctx.bufferless.stage = BufferlessStage::NeedSkippableHeaderRemainder(
                        size - dctx.bufferless.frame_bytes.len(),
                    );
                    forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)
                }
                frame::HeaderProbe::Header(header) => {
                    let payload_size = usize::try_from(header.frameContentSize)
                        .map_err(|_| ZSTD_ErrorCode::ZSTD_error_frameParameter_unsupported)?;
                    dctx.bufferless.header = Some(header);
                    dctx.bufferless.stage = if payload_size == 0 {
                        dctx.clear_once_dict();
                        BufferlessStage::Finished
                    } else {
                        BufferlessStage::NeedSkippablePayload(payload_size)
                    };
                    forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)
                }
            }
        }
        BufferlessStage::NeedBlockHeader => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            let written = forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)?;
            let header = parse_block_header(src)?;
            if header.content_size == 0 {
                if header.last_block {
                    if dctx.bufferless.header.expect("header set").checksumFlag != 0 {
                        dctx.bufferless.stage = BufferlessStage::NeedChecksum(4);
                    } else {
                        dctx.bufferless.stage = BufferlessStage::Finished;
                        dctx.clear_once_dict();
                        return Ok(written);
                    }
                } else {
                    dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
                }
                Ok(written)
            } else {
                dctx.bufferless.stage = BufferlessStage::NeedBlockBody(header);
                Ok(written)
            }
        }
        BufferlessStage::NeedBlockBody(_) => {
            decompress_block_continue(dctx, dst, dst_capacity, src, allow_staging)
        }
        BufferlessStage::NeedSkippablePayload(_) => {
            let written = forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)?;
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            dctx.bufferless.stage = BufferlessStage::Finished;
            dctx.clear_once_dict();
            Ok(written)
        }
        BufferlessStage::NeedChecksum(_) => {
            let written = forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)?;
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            dctx.bufferless.stage = BufferlessStage::Finished;
            dctx.clear_once_dict();
            Ok(written)
        }
        BufferlessStage::Finished => Ok(0),
    }
}

fn ensure_bufferless_upstream_session(
    dctx: &mut DecoderContext,
) -> Result<&mut UpstreamBufferlessSession, ZSTD_ErrorCode> {
    if dctx.bufferless.upstream.is_none() {
        dctx.bufferless.upstream = Some(UpstreamBufferlessSession::new(dctx)?);
    }
    Ok(dctx.bufferless.upstream.as_mut().expect("session initialized"))
}

fn forward_bufferless_chunk(
    dctx: &mut DecoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    src: &[u8],
    allow_staging: bool,
) -> Result<usize, ZSTD_ErrorCode> {
    if allow_staging {
        let mut block_output = vec![0u8; BLOCK_SIZE_MAX];
        let produced = ensure_bufferless_upstream_session(dctx)?.continue_decode(
            block_output.as_mut_ptr().cast(),
            block_output.len(),
            src,
        )?;
        let to_write = produced.min(dst_capacity);
        if to_write != 0 {
            unsafe {
                core::ptr::copy_nonoverlapping(block_output.as_ptr(), dst.cast::<u8>(), to_write);
            }
        }
        if to_write < produced {
            stage_decoded_output(dctx, &block_output[to_write..produced]);
        }
        Ok(to_write)
    } else {
        ensure_bufferless_upstream_session(dctx)?.continue_decode(dst, dst_capacity, src)
    }
}

fn replay_bufferless_prefix_with_upstream(dctx: &mut DecoderContext) -> Result<(), ZSTD_ErrorCode> {
    let mut session = UpstreamBufferlessSession::new(dctx)?;
    let mut scratch = vec![0u8; BLOCK_SIZE_MAX];
    let mut replayed = 0usize;
    while replayed < dctx.bufferless.frame_bytes.len() {
        let need = session.next_src_size_to_decompress()?;
        if need == 0 || replayed + need > dctx.bufferless.frame_bytes.len() {
            return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected);
        }
        session.continue_decode(
            scratch.as_mut_ptr().cast(),
            scratch.len(),
            &dctx.bufferless.frame_bytes[replayed..replayed + need],
        )?;
        replayed += need;
    }
    dctx.bufferless.upstream = Some(session);
    Ok(())
}

fn finish_bufferless_block(dctx: &mut DecoderContext, block: BlockHeader) {
    if block.last_block {
        if dctx.bufferless.header.expect("header set").checksumFlag != 0 {
            dctx.bufferless.stage = BufferlessStage::NeedChecksum(4);
        } else {
            dctx.bufferless.stage = BufferlessStage::Finished;
            dctx.clear_once_dict();
        }
    } else {
        dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
    }
}

fn decompress_block_continue(
    dctx: &mut DecoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    src: &[u8],
    allow_staging: bool,
) -> Result<usize, ZSTD_ErrorCode> {
    let BufferlessStage::NeedBlockBody(block) = dctx.bufferless.stage.clone() else {
        return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
    };
    let expected = if block.block_type == BlockType::Rle {
        1
    } else {
        block.content_size
    };
    if src.len() != expected {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    let written = forward_bufferless_chunk(dctx, dst, dst_capacity, src, allow_staging)?;
    dctx.bufferless.frame_bytes.extend_from_slice(src);
    finish_bufferless_block(dctx, block);
    Ok(written)
}

pub(crate) fn decompress_block_body(
    dctx: &mut DecoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    src: &[u8],
) -> Result<usize, ZSTD_ErrorCode> {
    let BufferlessStage::NeedBlockBody(block) = dctx.bufferless.stage.clone() else {
        return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
    };
    let expected = if block.block_type == BlockType::Rle {
        1
    } else {
        block.content_size
    };
    if src.len() != expected {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    let written = ensure_bufferless_upstream_session(dctx)?.decompress_block(dst, dst_capacity, src)?;
    dctx.bufferless.frame_bytes.extend_from_slice(src);
    dctx.bufferless.upstream = None;
    replay_bufferless_prefix_with_upstream(dctx)?;
    finish_bufferless_block(dctx, block);
    Ok(written)
}

pub(crate) fn stream_decompress(
    dctx: &mut DecoderContext,
    output: &mut crate::ffi::types::ZSTD_outBuffer,
    input: &mut crate::ffi::types::ZSTD_inBuffer,
) -> Result<usize, ZSTD_ErrorCode> {
    if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
        dctx.bufferless.reset();
        dctx.stream.reset();
    }

    if dctx.bufferless.upstream.is_none() {
        dctx.bufferless.upstream = Some(UpstreamBufferlessSession::new_stream(dctx)?);
        dctx.bufferless.stage = BufferlessStage::NeedStart;
    }

    let ret = dctx
        .bufferless
        .upstream
        .as_mut()
        .expect("stream session initialized")
        .decompress_stream(output, input)?;

    if ret == 0 {
        dctx.bufferless.upstream = None;
        dctx.bufferless.stage = BufferlessStage::Finished;
        dctx.stream.reset();
        dctx.clear_once_dict();
    } else {
        dctx.bufferless.stage = BufferlessStage::NeedStart;
    }

    Ok(ret)
}

pub(crate) fn dparam_bounds(param: ZSTD_dParameter) -> Option<(i32, i32)> {
    match param {
        ZSTD_dParameter::ZSTD_d_windowLogMax => Some((10, frame::ZSTD_WINDOWLOG_MAX as i32)),
        ZSTD_dParameter::ZSTD_d_experimentalParam1 => {
            Some((ZSTD_format_e::ZSTD_f_zstd1 as i32, ZSTD_format_e::ZSTD_f_zstd1_magicless as i32))
        }
        ZSTD_dParameter::ZSTD_d_experimentalParam2
        | ZSTD_dParameter::ZSTD_d_experimentalParam3
        | ZSTD_dParameter::ZSTD_d_experimentalParam4
        | ZSTD_dParameter::ZSTD_d_experimentalParam5 => Some((0, 1)),
    }
}

pub(crate) fn decoding_buffer_size_min(
    window_size: u64,
    frame_content_size: u64,
) -> Result<usize, ZSTD_ErrorCode> {
    let block_size = window_size.min(crate::decompress::block::BLOCK_SIZE_MAX as u64);
    let needed_rb_size = window_size
        .checked_add(block_size)
        .and_then(|value| value.checked_add(crate::decompress::block::BLOCK_SIZE_MAX as u64))
        .and_then(|value| value.checked_add((frame::WILDCOPY_OVERLENGTH * 2) as u64))
        .ok_or(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)?;
    let needed_size = frame_content_size.min(needed_rb_size);
    usize::try_from(needed_size).map_err(|_| ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)
}
