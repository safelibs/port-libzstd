use crate::{
    common::alloc,
    decompress::{
        block::{parse_block_header, BlockHeader, BlockType, BLOCK_HEADER_SIZE},
        frame::{self, DictionaryRef},
    },
    ffi::types::{
        ZSTD_DCtx, ZSTD_DDict, ZSTD_ErrorCode, ZSTD_dParameter,
        ZSTD_dictContentType_e, ZSTD_format_e,
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

fn stage_decoded_output(dctx: &mut DecoderContext, decoded: &[u8]) {
    dctx.stream.decoded.clear();
    dctx.stream.decoded.extend_from_slice(decoded);
    dctx.stream.output_pos = 0;
}

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

#[derive(Clone, Debug)]
struct BufferlessState {
    stage: BufferlessStage,
    frame_bytes: Vec<u8>,
    header: Option<crate::ffi::types::ZSTD_frameHeader>,
    decoded_bytes: usize,
}

impl Default for BufferlessState {
    fn default() -> Self {
        Self {
            stage: BufferlessStage::Idle,
            frame_bytes: Vec::new(),
            header: None,
            decoded_bytes: 0,
        }
    }
}

impl BufferlessState {
    fn begin(&mut self) {
        self.stage = BufferlessStage::NeedStart;
        self.frame_bytes.clear();
        self.header = None;
        self.decoded_bytes = 0;
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
                    Ok(0)
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
                    Ok(0)
                }
            }
        }
        BufferlessStage::NeedHeaderRemainder(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            match frame::parse_frame_header(&dctx.bufferless.frame_bytes, dctx.format)? {
                frame::HeaderProbe::Need(size) => {
                    dctx.bufferless.stage =
                        BufferlessStage::NeedHeaderRemainder(size - dctx.bufferless.frame_bytes.len());
                    Ok(0)
                }
                frame::HeaderProbe::Header(header) => {
                    dctx.bufferless.header = Some(header);
                    dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
                    Ok(0)
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
                    Ok(0)
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
                    Ok(0)
                }
            }
        }
        BufferlessStage::NeedBlockHeader => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            let header = parse_block_header(src)?;
            if header.content_size == 0 {
                if header.last_block {
                    if dctx.bufferless.header.expect("header set").checksumFlag != 0 {
                        dctx.bufferless.stage = BufferlessStage::NeedChecksum(4);
                    } else {
                        let decoded = frame::decode_all_frames(
                            &dctx.bufferless.frame_bytes,
                            dctx.resolved_dict()?,
                            dctx.format,
                            dctx.max_window_size,
                        )?;
                        if decoded.len() < dctx.bufferless.decoded_bytes {
                            return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected);
                        }
                        stage_decoded_output(dctx, &decoded[dctx.bufferless.decoded_bytes..]);
                        let written = drain_staged_output(dctx, dst, dst_capacity)?;
                        dctx.bufferless.stage = BufferlessStage::Finished;
                        dctx.bufferless.decoded_bytes = decoded.len();
                        dctx.clear_once_dict();
                        return Ok(written);
                    }
                } else {
                    dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
                }
                Ok(0)
            } else {
                dctx.bufferless.stage = BufferlessStage::NeedBlockBody(header);
                Ok(0)
            }
        }
        BufferlessStage::NeedBlockBody(_) => decompress_block_body(dctx, dst, dst_capacity, src),
        BufferlessStage::NeedSkippablePayload(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            dctx.bufferless.stage = BufferlessStage::Finished;
            dctx.clear_once_dict();
            Ok(0)
        }
        BufferlessStage::NeedChecksum(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            complete_bufferless_frame(dctx, dst, dst_capacity)
        }
        BufferlessStage::Finished => Ok(0),
    }
}

fn complete_bufferless_frame(
    dctx: &mut DecoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
) -> Result<usize, ZSTD_ErrorCode> {
    let completed_output = frame::decode_all_frames(
        &dctx.bufferless.frame_bytes,
        dctx.resolved_dict()?,
        dctx.format,
        dctx.max_window_size,
    )?;
    if completed_output.len() < dctx.bufferless.decoded_bytes {
        return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected);
    }

    stage_decoded_output(dctx, &completed_output[dctx.bufferless.decoded_bytes..]);
    let written = drain_staged_output(dctx, dst, dst_capacity)?;
    dctx.bufferless.decoded_bytes = completed_output.len();
    dctx.bufferless.stage = BufferlessStage::Finished;
    dctx.clear_once_dict();
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

    dctx.bufferless.frame_bytes.extend_from_slice(src);

    if block.last_block {
        if dctx.bufferless.header.expect("header set").checksumFlag != 0 {
            dctx.bufferless.stage = BufferlessStage::NeedChecksum(4);
            return Ok(0);
        }
        return complete_bufferless_frame(dctx, dst, dst_capacity);
    }

    dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
    Ok(0)
}

pub(crate) fn stream_decompress(
    dctx: &mut DecoderContext,
    output: &mut crate::ffi::types::ZSTD_outBuffer,
    input: &mut crate::ffi::types::ZSTD_inBuffer,
) -> Result<usize, ZSTD_ErrorCode> {
    let initial_input_pos = input.pos;
    let initial_output_pos = output.pos;
    let initial_stage = dctx.bufferless.stage.clone();
    let initial_compressed_len = dctx.stream.compressed.len();
    let initial_staged_len = dctx.stream.decoded.len();
    let initial_staged_pos = dctx.stream.output_pos;
    let initial_deferred = dctx.stream.deferred_input_advance;

    if !dctx.stream.decoded.is_empty() {
        let writable = output.size.saturating_sub(output.pos);
        let dst_ptr = (output.dst as *mut u8).wrapping_add(output.pos);
        let written = drain_staged_output(dctx, dst_ptr.cast(), writable)?;
        output.pos += written;
        if dctx.stream.decoded.is_empty() && dctx.stream.deferred_input_advance != 0 {
            let available = input.size.saturating_sub(input.pos);
            if available < dctx.stream.deferred_input_advance {
                return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
            }
            input.pos += dctx.stream.deferred_input_advance;
            dctx.stream.deferred_input_advance = 0;
        }
        if !dctx.stream.decoded.is_empty() {
            return Ok(1);
        }
        if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
            dctx.bufferless.reset();
            dctx.stream.compressed.clear();
            return Ok(0);
        }
        if written > 0 && output.pos == output.size {
            return Ok(dctx
                .bufferless
                .next_src_size(dctx.format)
                .saturating_sub(dctx.stream.compressed.len()));
        }
    }

    let input_pos_before_loop = input.pos;

    if matches!(dctx.bufferless.stage, BufferlessStage::Idle) {
        begin_bufferless(dctx);
        dctx.stream.compressed.clear();
    }

    loop {
        if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
            dctx.bufferless.reset();
            dctx.stream.compressed.clear();
            return Ok(0);
        }

        let writable = output.size.saturating_sub(output.pos);

        let needed = dctx.bufferless.next_src_size(dctx.format);
        if needed == 0 {
            break;
        }

        let missing = needed.saturating_sub(dctx.stream.compressed.len());
        let src_remaining = input.size.saturating_sub(input.pos);
        let to_take = missing.min(src_remaining);
        if to_take > 0 {
            let src_ptr = (input.src as *const u8).wrapping_add(input.pos);
            // SAFETY: `input` points at a readable buffer provided by the caller.
            let chunk = unsafe { core::slice::from_raw_parts(src_ptr, to_take) };
            dctx.stream.compressed.extend_from_slice(chunk);
            input.pos += to_take;
        }

        if matches!(dctx.bufferless.stage, BufferlessStage::NeedStart)
            && !frame::partial_frame_prefix_is_valid(&dctx.stream.compressed, dctx.format)
        {
            return Err(ZSTD_ErrorCode::ZSTD_error_prefix_unknown);
        }

        if dctx.stream.compressed.len() < needed {
            break;
        }

        let chunk = core::mem::take(&mut dctx.stream.compressed);
        let dst_ptr = (output.dst as *mut u8).wrapping_add(output.pos);
        let written = bufferless_continue(dctx, dst_ptr.cast(), writable, &chunk)?;
        output.pos += written;

        if !dctx.stream.decoded.is_empty() {
            let consumed = input.pos.saturating_sub(input_pos_before_loop);
            if consumed != 0 {
                let to_defer = if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
                    1.min(consumed)
                } else {
                    consumed
                };
                dctx.stream.deferred_input_advance += to_defer;
                input.pos -= to_defer;
            }
            return Ok(1);
        }
        if written > 0 && output.pos == output.size {
            break;
        }

        if input.pos == input.size && written == 0 {
            break;
        }
    }

    let state_changed = dctx.bufferless.stage != initial_stage
        || dctx.stream.compressed.len() != initial_compressed_len
        || dctx.stream.decoded.len() != initial_staged_len
        || dctx.stream.output_pos != initial_staged_pos
        || dctx.stream.deferred_input_advance != initial_deferred;

    if input.pos == initial_input_pos && output.pos == initial_output_pos && !state_changed {
        if input.pos == input.size
            && !matches!(
                dctx.bufferless.stage,
                BufferlessStage::Idle | BufferlessStage::Finished
            )
        {
            return Ok(dctx
                .bufferless
                .next_src_size(dctx.format)
                .saturating_sub(dctx.stream.compressed.len()));
        }
        return Err(if input.pos == input.size {
            ZSTD_ErrorCode::ZSTD_error_noForwardProgress_inputEmpty
        } else {
            ZSTD_ErrorCode::ZSTD_error_noForwardProgress_destFull
        });
    }

    if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
        dctx.bufferless.reset();
        dctx.stream.compressed.clear();
        Ok(0)
    } else {
        Ok(dctx
            .bufferless
            .next_src_size(dctx.format)
            .saturating_sub(dctx.stream.compressed.len()))
    }
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
