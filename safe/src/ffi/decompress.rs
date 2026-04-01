use crate::{
    common::alloc,
    decompress::{
        block::{parse_block_header, BlockHeader, BlockType, BLOCK_HEADER_SIZE},
        frame::{self, DictionaryRef},
        legacy,
    },
    ffi::types::{
        ZSTD_DCtx, ZSTD_DDict, ZSTD_ErrorCode, ZSTD_dParameter, ZSTD_dictContentType_e,
        ZSTD_dictLoadMethod_e, ZSTD_format_e, ZSTD_inBuffer, ZSTD_outBuffer,
    },
};
use core::{ffi::c_void, mem::size_of};

fn validate_formatted_dictionary(bytes: &[u8]) -> Result<(), ZSTD_ErrorCode> {
    if bytes.len() < 8 {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
    }
    if structured_zstd::decoding::Dictionary::decode_dict(bytes).is_ok()
        || crate::dict_builder::zdict::formatted_dictionary_content(bytes).is_ok()
    {
        Ok(())
    } else {
        Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted)
    }
}

#[derive(Clone, Debug)]
enum DecoderDictionaryStorage {
    Owned(Vec<u8>),
    Referenced(*const u8, usize),
}

impl DecoderDictionaryStorage {
    fn owned(bytes: &[u8]) -> Self {
        Self::Owned(bytes.to_vec())
    }

    fn referenced(ptr: *const u8, len: usize) -> Self {
        Self::Referenced(ptr, len)
    }

    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes.as_slice(),
            Self::Referenced(ptr, len) => unsafe { core::slice::from_raw_parts(*ptr, *len) },
        }
    }

    fn heap_size(&self) -> usize {
        match self {
            Self::Owned(bytes) => alloc::heap_bytes(bytes.len()),
            Self::Referenced(_, _) => 0,
        }
    }
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

    fn dict_id(&self) -> Result<u32, ZSTD_ErrorCode> {
        match self {
            DictionarySelection::None => Ok(0),
            DictionarySelection::Referenced(ptr) => ddict_ref(*ptr)
                .map(|ddict| ddict.dict_id)
                .ok_or(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong),
            DictionarySelection::Owned { dict_id, .. } => Ok(*dict_id),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DecoderDictionary {
    storage: DecoderDictionaryStorage,
    pub dict_id: u32,
    pub formatted: bool,
    static_workspace_size: usize,
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
            storage: DecoderDictionaryStorage::owned(bytes),
            dict_id: if formatted {
                crate::decompress::fse::formatted_dict_id(bytes)
            } else {
                0
            },
            formatted,
            static_workspace_size: 0,
        })
    }

    fn from_storage(
        storage: DecoderDictionaryStorage,
        dict_content_type: ZSTD_dictContentType_e,
        static_workspace_size: usize,
    ) -> Result<Self, ZSTD_ErrorCode> {
        let bytes = storage.as_slice();
        let mut ddict = Self::from_bytes_with_content_type(bytes, dict_content_type)?;
        ddict.storage = storage;
        ddict.static_workspace_size = static_workspace_size;
        Ok(ddict)
    }

    pub(crate) fn as_dictionary_ref(&self) -> DictionaryRef<'_> {
        if self.formatted {
            let _ = self.dict_id;
            DictionaryRef::Formatted(self.storage.as_slice())
        } else {
            DictionaryRef::Raw(self.storage.as_slice())
        }
    }

    pub(crate) fn heap_size(&self) -> usize {
        self.storage.heap_size()
    }

    pub(crate) fn workspace_size(&self) -> usize {
        self.static_workspace_size
    }
}

#[derive(Clone, Debug, Default)]
struct StreamState {
    compressed: Vec<u8>,
    decoded: Vec<u8>,
    output_pos: usize,
    deferred_input_advance: usize,
    finished_returned: bool,
    legacy_context: *mut c_void,
    legacy_version: u32,
}

impl StreamState {
    fn reset(&mut self) {
        self.compressed.clear();
        self.decoded.clear();
        self.output_pos = 0;
        self.deferred_input_advance = 0;
        self.finished_returned = false;
        self.legacy_context = core::ptr::null_mut();
        self.legacy_version = 0;
    }

    fn is_busy(&self) -> bool {
        !self.compressed.is_empty()
            || self.output_pos < self.decoded.len()
            || self.deferred_input_advance != 0
            || !self.legacy_context.is_null()
            || self.legacy_version != 0
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
    decoded_prefix: Vec<u8>,
}

impl Default for BufferlessState {
    fn default() -> Self {
        Self {
            stage: BufferlessStage::Idle,
            frame_bytes: Vec::new(),
            header: None,
            decoded_prefix: Vec::new(),
        }
    }
}

impl Clone for BufferlessState {
    fn clone(&self) -> Self {
        Self {
            stage: self.stage.clone(),
            frame_bytes: self.frame_bytes.clone(),
            header: self.header,
            decoded_prefix: self.decoded_prefix.clone(),
        }
    }
}

impl BufferlessState {
    fn begin(&mut self) {
        self.stage = BufferlessStage::NeedStart;
        self.frame_bytes.clear();
        self.header = None;
        self.decoded_prefix.clear();
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn is_busy(&self) -> bool {
        !matches!(
            self.stage,
            BufferlessStage::Idle | BufferlessStage::Finished
        )
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

    fn next_input_type(&self) -> crate::ffi::types::ZSTD_nextInputType_e {
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
        alloc::heap_bytes(self.frame_bytes.len() + self.decoded_prefix.len())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DecoderContext {
    pub(crate) static_workspace_size: usize,
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
            static_workspace_size: 0,
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
        self.static_workspace_size.max(alloc::base_size::<Self>())
            + match &self.dict {
                DictionarySelection::None => 0,
                DictionarySelection::Referenced(ptr) => {
                    ddict_ref(*ptr).map_or(0, DecoderDictionary::heap_size)
                }
                DictionarySelection::Owned { raw, .. } => alloc::heap_bytes(raw.len()),
            }
            + self.stream.size_of()
            + self.bufferless.size_of()
    }

    pub(crate) fn mark_static(&mut self, workspace_size: usize) {
        self.static_workspace_size = workspace_size;
    }

    pub(crate) fn can_set_parameters(&self) -> bool {
        !self.stream.is_busy() && !self.bufferless.is_busy()
    }

    pub(crate) fn reset_session(&mut self) {
        self.release_legacy_stream();
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
        let static_workspace_size = self.static_workspace_size;
        self.format = other.format;
        self.max_window_size = other.max_window_size;
        self.stable_out_buffer = other.stable_out_buffer;
        self.force_ignore_checksum = other.force_ignore_checksum;
        self.ref_multiple_ddicts = other.ref_multiple_ddicts;
        self.disable_huffman_assembly = other.disable_huffman_assembly;
        self.dict = other.dict.clone();
        self.reset_session();
        self.static_workspace_size = static_workspace_size;
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
            raw: ddict.storage.as_slice().to_vec(),
            formatted: ddict.formatted,
            dict_id: ddict.dict_id,
            use_mode,
        };
        Ok(())
    }

    pub(crate) fn ref_ddict(
        &mut self,
        ddict: *const DecoderDictionary,
    ) -> Result<(), ZSTD_ErrorCode> {
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

    pub(crate) fn resolved_dict_id(&self) -> Result<u32, ZSTD_ErrorCode> {
        self.dict.dict_id()
    }

    pub(crate) fn clear_once_dict(&mut self) {
        self.dict.consume_once();
    }

    pub(crate) fn release_legacy_stream(&mut self) {
        let _ = legacy::free_stream_context(self.stream.legacy_context, self.stream.legacy_version);
        self.stream.legacy_context = core::ptr::null_mut();
        self.stream.legacy_version = 0;
    }

    pub(crate) fn set_parameter(
        &mut self,
        param: ZSTD_dParameter,
        mut value: i32,
    ) -> Result<(), ZSTD_ErrorCode> {
        if !self.can_set_parameters() {
            return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
        }

        let (lower, upper) =
            dparam_bounds(param).ok_or(ZSTD_ErrorCode::ZSTD_error_parameter_unsupported)?;
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

    pub(crate) fn set_max_window_size(
        &mut self,
        max_window_size: usize,
    ) -> Result<(), ZSTD_ErrorCode> {
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
            raw: ddict.storage.as_slice().to_vec(),
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

pub(crate) fn init_static_dctx(workspace: *mut c_void, workspace_size: usize) -> *mut ZSTD_DCtx {
    if workspace.is_null()
        || workspace_size < size_of::<DecoderContext>()
        || (workspace as usize & 7) != 0
    {
        return core::ptr::null_mut();
    }
    let ptr = workspace.cast::<DecoderContext>();
    unsafe {
        ptr.write(DecoderContext::default());
        (*ptr).mark_static(workspace_size);
    }
    ptr.cast()
}

pub(crate) fn free_dctx(ptr: *mut ZSTD_DCtx) -> usize {
    if ptr.is_null() {
        return 0;
    }
    if dctx_ref(ptr.cast_const()).is_some_and(|dctx| dctx.static_workspace_size != 0) {
        return crate::common::error::error_result(ZSTD_ErrorCode::ZSTD_error_memory_allocation);
    }
    if let Some(dctx) = dctx_mut(ptr) {
        dctx.release_legacy_stream();
    }
    // SAFETY: `ptr` originated from `create_dctx`.
    unsafe {
        drop(Box::from_raw(ptr.cast::<DecoderContext>()));
    }
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

pub(crate) fn init_static_ddict(
    workspace: *mut c_void,
    workspace_size: usize,
    dict: &[u8],
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
) -> Result<*mut ZSTD_DDict, ZSTD_ErrorCode> {
    let needed = alloc::base_size::<DecoderDictionary>()
        + if matches!(dict_load_method, ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy) {
            dict.len()
        } else {
            0
        };
    if workspace.is_null() || workspace_size < needed || (workspace as usize & 7) != 0 {
        return Err(ZSTD_ErrorCode::ZSTD_error_memory_allocation);
    }

    let storage = match dict_load_method {
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy => {
            let dict_ptr = unsafe { workspace.cast::<u8>().add(size_of::<DecoderDictionary>()) };
            unsafe {
                core::ptr::copy_nonoverlapping(dict.as_ptr(), dict_ptr, dict.len());
            }
            DecoderDictionaryStorage::referenced(dict_ptr, dict.len())
        }
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef => {
            DecoderDictionaryStorage::referenced(dict.as_ptr(), dict.len())
        }
    };

    let ddict = DecoderDictionary::from_storage(storage, dict_content_type, workspace_size)?;
    let ptr = workspace.cast::<DecoderDictionary>();
    unsafe {
        ptr.write(ddict);
    }
    Ok(ptr.cast())
}

pub(crate) fn free_ddict(ptr: *mut ZSTD_DDict) -> usize {
    if ptr.is_null() {
        return 0;
    }
    if ddict_ref(ptr.cast()).is_some_and(|ddict| ddict.workspace_size() != 0) {
        return 0;
    }
    // SAFETY: `ptr` originated from `create_ddict`.
    unsafe {
        drop(Box::from_raw(ptr.cast::<DecoderDictionary>()));
    }
    0
}

pub(crate) fn sizeof_dctx(ptr: *const ZSTD_DCtx) -> usize {
    dctx_ref(ptr).map_or(0, DecoderContext::sizeof)
}

pub(crate) fn sizeof_ddict(ptr: *const ZSTD_DDict) -> usize {
    ddict_ref(ptr.cast()).map_or(0, |ddict| {
        ddict
            .workspace_size()
            .max(alloc::base_size::<DecoderDictionary>())
            + ddict.heap_size()
    })
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

pub(crate) fn next_input_type(ptr: *mut ZSTD_DCtx) -> crate::ffi::types::ZSTD_nextInputType_e {
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
                && matches!(
                    frame::classify_frame(&dctx.bufferless.frame_bytes),
                    Some(frame::FrameKind::Legacy(_))
                )
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
                        BufferlessStage::NeedHeaderRemainder(
                            size - dctx.bufferless.frame_bytes.len(),
                        )
                    };
                    let _ = (dst, dst_capacity, allow_staging);
                    Ok(0)
                }
                frame::HeaderProbe::Header(header) => {
                    validate_frame_dictionary(dctx, header)?;
                    dctx.bufferless.header = Some(header);
                    if header.frameType == crate::ffi::types::ZSTD_frameType_e::ZSTD_skippableFrame
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
                    let _ = (dst, dst_capacity, allow_staging);
                    Ok(0)
                }
            }
        }
        BufferlessStage::NeedHeaderRemainder(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            match frame::parse_frame_header(&dctx.bufferless.frame_bytes, dctx.format)? {
                frame::HeaderProbe::Need(size) => {
                    dctx.bufferless.stage = BufferlessStage::NeedHeaderRemainder(
                        size - dctx.bufferless.frame_bytes.len(),
                    );
                    let _ = (dst, dst_capacity, allow_staging);
                    Ok(0)
                }
                frame::HeaderProbe::Header(header) => {
                    validate_frame_dictionary(dctx, header)?;
                    dctx.bufferless.header = Some(header);
                    dctx.bufferless.stage = BufferlessStage::NeedBlockHeader;
                    let _ = (dst, dst_capacity, allow_staging);
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
                    let _ = (dst, dst_capacity, allow_staging);
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
                    let _ = (dst, dst_capacity, allow_staging);
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
                        dctx.bufferless.stage = BufferlessStage::Finished;
                        dctx.clear_once_dict();
                        return Ok(0);
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
        BufferlessStage::NeedBlockBody(_) => {
            decompress_block_continue(dctx, dst, dst_capacity, src, allow_staging)
        }
        BufferlessStage::NeedSkippablePayload(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            dctx.bufferless.stage = BufferlessStage::Finished;
            dctx.clear_once_dict();
            let _ = (dst, dst_capacity, allow_staging);
            Ok(0)
        }
        BufferlessStage::NeedChecksum(_) => {
            dctx.bufferless.frame_bytes.extend_from_slice(src);
            dctx.bufferless.stage = BufferlessStage::Finished;
            dctx.clear_once_dict();
            let _ = (dst, dst_capacity, allow_staging);
            Ok(0)
        }
        BufferlessStage::Finished => Ok(0),
    }
}

fn validate_frame_dictionary(
    dctx: &DecoderContext,
    header: crate::ffi::types::ZSTD_frameHeader,
) -> Result<(), ZSTD_ErrorCode> {
    if header.dictID != 0 && dctx.resolved_dict_id()? != header.dictID {
        return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong);
    }
    Ok(())
}

fn write_bufferless_output(
    dctx: &mut DecoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    decoded: &[u8],
    allow_staging: bool,
) -> Result<usize, ZSTD_ErrorCode> {
    if decoded.is_empty() {
        return Ok(0);
    }

    if allow_staging {
        let to_write = decoded.len().min(dst_capacity);
        if to_write != 0 {
            if dst.is_null() {
                return Err(ZSTD_ErrorCode::ZSTD_error_dstBuffer_null);
            }
            // SAFETY: `dst` points to `dst_capacity` writable bytes.
            unsafe {
                core::ptr::copy_nonoverlapping(decoded.as_ptr(), dst.cast::<u8>(), to_write);
            }
        }
        if to_write < decoded.len() {
            stage_decoded_output(dctx, &decoded[to_write..]);
        }
        Ok(to_write)
    } else {
        let written = frame::copy_decoded_to_ptr(decoded, dst, dst_capacity);
        frame::decode_error_result(written)
    }
}

fn decode_bufferless_prefix(dctx: &DecoderContext, src: &[u8]) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let header = dctx
        .bufferless
        .header
        .ok_or(ZSTD_ErrorCode::ZSTD_error_corruption_detected)?;
    let synthetic =
        frame::build_synthetic_block_frame(&dctx.bufferless.frame_bytes, header, dctx.format, src)?;
    frame::decode_all_frames_relaxed(
        &synthetic,
        dctx.resolved_dict()?,
        dctx.format,
        dctx.max_window_size,
    )
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

    let decoded = decode_bufferless_prefix(dctx, src)?;
    let prefix_len = dctx.bufferless.decoded_prefix.len();
    if decoded.len() < prefix_len || !decoded.starts_with(&dctx.bufferless.decoded_prefix) {
        return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected);
    }
    let written = write_bufferless_output(
        dctx,
        dst,
        dst_capacity,
        &decoded[prefix_len..],
        allow_staging,
    )?;
    dctx.bufferless.decoded_prefix = decoded;
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
    decompress_block_continue(dctx, dst, dst_capacity, src, false)
}

pub(crate) fn insert_uncompressed_block(
    dctx: &mut DecoderContext,
    block: &[u8],
) -> Result<usize, ZSTD_ErrorCode> {
    let BufferlessStage::NeedBlockBody(header) = dctx.bufferless.stage.clone() else {
        return Err(ZSTD_ErrorCode::ZSTD_error_stage_wrong);
    };
    if header.block_type != BlockType::Raw {
        return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected);
    }
    if header.content_size != block.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    dctx.bufferless.decoded_prefix.extend_from_slice(block);
    dctx.bufferless.frame_bytes.extend_from_slice(block);
    finish_bufferless_block(dctx, header);
    Ok(block.len())
}

fn output_remaining(output: &ZSTD_outBuffer) -> usize {
    output.size.saturating_sub(output.pos)
}

fn output_ptr(output: &mut ZSTD_outBuffer) -> *mut c_void {
    if output_remaining(output) == 0 || output.dst.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: `output.pos < output.size`, so the pointer arithmetic stays in-bounds.
    unsafe { (output.dst.cast::<u8>()).add(output.pos).cast() }
}

fn apply_deferred_input_advance(dctx: &mut DecoderContext, input: &mut ZSTD_inBuffer) {
    if dctx.stream.deferred_input_advance == 0 {
        return;
    }

    let remaining = input.size.saturating_sub(input.pos);
    let advance = dctx.stream.deferred_input_advance.min(remaining);
    input.pos = input.pos.saturating_add(advance);
    dctx.stream.deferred_input_advance -= advance;
}

fn hide_visible_input_progress(
    dctx: &mut DecoderContext,
    input: &mut ZSTD_inBuffer,
    visible_input_pos: usize,
) {
    let hidden = input.pos.saturating_sub(visible_input_pos);
    dctx.stream.deferred_input_advance = dctx.stream.deferred_input_advance.saturating_add(hidden);
    input.pos = visible_input_pos;
}

fn dictionary_bytes(dict: DictionaryRef<'_>) -> Option<&[u8]> {
    match dict {
        DictionaryRef::None => None,
        DictionaryRef::Raw(bytes) | DictionaryRef::Formatted(bytes) => Some(bytes),
    }
}

fn stream_hint(dctx: &DecoderContext) -> usize {
    dctx.bufferless
        .next_src_size(dctx.format)
        .saturating_sub(dctx.stream.compressed.len())
}

fn try_stream_legacy(
    dctx: &mut DecoderContext,
    output: &mut ZSTD_outBuffer,
    input: &mut ZSTD_inBuffer,
) -> Result<Option<usize>, ZSTD_ErrorCode> {
    if !matches!(
        dctx.bufferless.stage,
        BufferlessStage::Idle
            | BufferlessStage::NeedStart
            | BufferlessStage::NeedHeaderRemainder(_)
    ) && dctx.stream.legacy_version == 0
    {
        return Ok(None);
    }

    let Some(full_input) = optional_src_slice(input.src, input.size) else {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    let remaining = &full_input[input.pos..];

    if dctx.stream.legacy_version == 0 {
        let candidate = if dctx.stream.compressed.is_empty() {
            remaining
        } else {
            dctx.stream.compressed.as_slice()
        };
        let Some(version) = legacy::supported_version(candidate) else {
            return Ok(None);
        };
        let dict_storage = dictionary_bytes(dctx.resolved_dict()?).map(|bytes| bytes.to_vec());
        legacy::init_stream_context(
            &mut dctx.stream.legacy_context,
            dctx.stream.legacy_version,
            version,
            dict_storage.as_deref(),
        )?;
        dctx.stream.legacy_version = version;
    }

    let buffered = dctx.stream.compressed.clone();
    let old_len = buffered.len();
    let mut combined = buffered;
    combined.extend_from_slice(remaining);
    let mut legacy_input = ZSTD_inBuffer {
        src: if combined.is_empty() {
            core::ptr::null()
        } else {
            combined.as_ptr().cast()
        },
        size: combined.len(),
        pos: 0,
    };
    let hint = legacy::decompress_stream(
        dctx.stream.legacy_context,
        dctx.stream.legacy_version,
        output,
        &mut legacy_input,
    )?;
    let consumed = legacy_input.pos;
    let consumed_from_current = consumed.saturating_sub(old_len);
    dctx.stream.compressed.clear();
    if consumed < old_len {
        dctx.stream
            .compressed
            .extend_from_slice(&combined[consumed..old_len]);
    }
    input.pos = input.pos.saturating_add(consumed_from_current);
    if hint == 0 {
        dctx.bufferless.stage = BufferlessStage::Finished;
        dctx.bufferless.frame_bytes.clear();
        dctx.bufferless.header = None;
        dctx.bufferless.decoded_prefix.clear();
        dctx.release_legacy_stream();
        dctx.clear_once_dict();
    } else {
        dctx.bufferless.stage = BufferlessStage::NeedStart;
    }
    Ok(Some(hint))
}

pub(crate) fn stream_decompress(
    dctx: &mut DecoderContext,
    output: &mut crate::ffi::types::ZSTD_outBuffer,
    input: &mut crate::ffi::types::ZSTD_inBuffer,
) -> Result<usize, ZSTD_ErrorCode> {
    let visible_input_pos = input.pos;

    if output_remaining(output) == 0 {
        if matches!(dctx.bufferless.stage, BufferlessStage::Finished)
            && dctx.stream.decoded.is_empty()
        {
            return Ok(0);
        }
        if !dctx.stream.decoded.is_empty()
            || matches!(dctx.bufferless.stage, BufferlessStage::NeedBlockBody(_))
        {
            return Err(ZSTD_ErrorCode::ZSTD_error_noForwardProgress_destFull);
        }
    }

    if dctx.stream.decoded.is_empty() {
        apply_deferred_input_advance(dctx, input);
    }

    if !dctx.stream.decoded.is_empty() {
        if output_remaining(output) == 0 {
            if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
                return Ok(0);
            }
            hide_visible_input_progress(dctx, input, visible_input_pos);
            return Ok(stream_hint(dctx).max(1));
        }
        output.pos += drain_staged_output(dctx, output_ptr(output), output_remaining(output))?;
        if !dctx.stream.decoded.is_empty() {
            hide_visible_input_progress(dctx, input, visible_input_pos);
            return Ok(stream_hint(dctx).max(1));
        }
        apply_deferred_input_advance(dctx, input);
        if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
            return Ok(0);
        }
        if output_remaining(output) == 0 {
            return Ok(stream_hint(dctx).max(1));
        }
    }

    if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
        if !dctx.stream.finished_returned {
            dctx.stream.finished_returned = true;
            return Ok(0);
        }
        if input.pos == input.size {
            return Ok(0);
        }
        dctx.bufferless.reset();
        dctx.stream.reset();
    }

    if matches!(dctx.bufferless.stage, BufferlessStage::Idle) {
        begin_bufferless(dctx);
    }

    if let Some(ret) = try_stream_legacy(dctx, output, input)? {
        return Ok(ret);
    }

    loop {
        let need = dctx.bufferless.next_src_size(dctx.format);
        if need == 0 {
            return Ok(0);
        }

        if dctx.stream.compressed.len() < need {
            let available = input.size.saturating_sub(input.pos);
            if available == 0 {
                return Ok(need - dctx.stream.compressed.len());
            }
            let to_take = (need - dctx.stream.compressed.len()).min(available);
            let Some(src) = optional_src_slice(input.src, input.size) else {
                return Err(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
            };
            dctx.stream
                .compressed
                .extend_from_slice(&src[input.pos..input.pos + to_take]);
            input.pos = input.pos.saturating_add(to_take);
            if dctx.stream.compressed.len() < need {
                if matches!(
                    dctx.bufferless.stage,
                    BufferlessStage::NeedStart
                        | BufferlessStage::NeedHeaderRemainder(_)
                        | BufferlessStage::NeedSkippableHeaderRemainder(_)
                ) {
                    let mut partial_prefix = dctx.bufferless.frame_bytes.clone();
                    partial_prefix.extend_from_slice(&dctx.stream.compressed);
                    if !frame::partial_frame_prefix_is_valid(&partial_prefix, dctx.format) {
                        return Err(ZSTD_ErrorCode::ZSTD_error_prefix_unknown);
                    }
                }
                apply_deferred_input_advance(dctx, input);
                return Ok(need - dctx.stream.compressed.len());
            }
        }

        if let Some(ret) = try_stream_legacy(dctx, output, input)? {
            return Ok(ret);
        }

        let chunk = dctx.stream.compressed.clone();
        let produced = bufferless_continue(
            dctx,
            output_ptr(output),
            output_remaining(output),
            &chunk,
            true,
        )?;
        dctx.stream.compressed.clear();
        output.pos += produced;

        if !dctx.stream.decoded.is_empty() {
            hide_visible_input_progress(dctx, input, visible_input_pos);
            return Ok(stream_hint(dctx).max(1));
        }
        apply_deferred_input_advance(dctx, input);
        if matches!(dctx.bufferless.stage, BufferlessStage::Finished) {
            return Ok(0);
        }
        if output_remaining(output) == 0 {
            if matches!(dctx.bufferless.stage, BufferlessStage::NeedChecksum(_))
                && input.pos < input.size
            {
                continue;
            }
            return Ok(stream_hint(dctx).max(1));
        }
        if input.pos == input.size {
            return Ok(stream_hint(dctx).max(1));
        }
    }
}

pub(crate) fn dparam_bounds(param: ZSTD_dParameter) -> Option<(i32, i32)> {
    match param {
        ZSTD_dParameter::ZSTD_d_windowLogMax => Some((10, frame::ZSTD_WINDOWLOG_MAX as i32)),
        ZSTD_dParameter::ZSTD_d_experimentalParam1 => Some((
            ZSTD_format_e::ZSTD_f_zstd1 as i32,
            ZSTD_format_e::ZSTD_f_zstd1_magicless as i32,
        )),
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
    usize::try_from(needed_size)
        .map_err(|_| ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)
}
