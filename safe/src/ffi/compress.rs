use crate::{
    common::{
        alloc,
        error::{error_result, is_error_result},
    },
    decompress::{
        ddict,
        frame::{parse_frame_header, HeaderProbe},
        huf::is_formatted_dictionary,
    },
    ffi::types::{
        ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CDict, ZSTD_ErrorCode, ZSTD_ResetDirective,
        ZSTD_Sequence, ZSTD_bounds, ZSTD_cParameter, ZSTD_compressionParameters, ZSTD_customMem,
        ZSTD_dParameter, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e, ZSTD_format_e,
        ZSTD_frameParameters, ZSTD_inBuffer, ZSTD_outBuffer, ZSTD_parameters,
        ZSTD_sequenceFormat_e, ZSTD_sequenceProducer_F, ZSTD_strategy, ZSTD_threadPool,
        ZSTD_BLOCKSIZE_MAX, ZSTD_CLEVEL_DEFAULT, ZSTD_CONTENTSIZE_UNKNOWN,
    },
};
use core::{
    cmp::min,
    ffi::{c_int, c_void},
    mem::size_of,
};
use oxiarc_zstd::{
    LevelConfig as OxiarcLevelConfig, MatchFinder as OxiarcMatchFinder, ZstdEncoder,
};
use std::{borrow::Cow, vec::Vec};
use structured_zstd::decoding::Dictionary as StructuredDictionary;
use structured_zstd::encoding::{
    CompressionLevel as StructuredCompressionLevel, FrameCompressor, Matcher,
    Sequence as StructuredSequence,
};

const ZSTD_MAGICNUMBER: u32 = 0xFD2F_B528;
const BLOCK_HEADER_SIZE: usize = 3;
const XXH64_SEED: u64 = 0;
const ZSTD_MAX_CLEVEL: c_int = 22;
const ZSTD_MIN_CLEVEL: c_int = -(ZSTD_BLOCKSIZE_MAX as c_int);
const XXH64_PRIME_1: u64 = 0x9E37_79B1_85EB_CA87;
const XXH64_PRIME_2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const XXH64_PRIME_3: u64 = 0x1656_67B1_9E37_79F9;
const XXH64_PRIME_4: u64 = 0x85EB_CA77_C2B2_AE63;
const XXH64_PRIME_5: u64 = 0x27D4_EB2F_1656_67C5;

#[derive(Clone, Debug)]
enum EncoderDictionaryStorage {
    Owned(Vec<u8>),
    Referenced(*const u8, usize),
}

impl EncoderDictionaryStorage {
    fn from_load_method(bytes: &[u8], dict_load_method: ZSTD_dictLoadMethod_e) -> Self {
        match dict_load_method {
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy => Self::Owned(bytes.to_vec()),
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef => Self::Referenced(bytes.as_ptr(), bytes.len()),
        }
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

#[derive(Clone, Debug)]
pub(crate) struct EncoderDictionary {
    storage: EncoderDictionaryStorage,
    static_workspace_size: usize,
    pub(crate) dict_id: u32,
    pub(crate) compression_level: c_int,
    pub(crate) cparams: ZSTD_compressionParameters,
    pub(crate) enable_long_distance_matching: bool,
    pub(crate) nb_workers: c_int,
    pub(crate) job_size: c_int,
    pub(crate) overlap_log: c_int,
    pub(crate) block_delimiters: ZSTD_sequenceFormat_e,
    pub(crate) validate_sequences: bool,
    pub(crate) enable_seq_producer_fallback: bool,
    pub(crate) dict_content_type: ZSTD_dictContentType_e,
}

impl EncoderDictionary {
    fn from_storage(
        storage: EncoderDictionaryStorage,
        bytes: &[u8],
        compression_level: c_int,
        cparams: ZSTD_compressionParameters,
        enable_long_distance_matching: bool,
        nb_workers: c_int,
        job_size: c_int,
        overlap_log: c_int,
        block_delimiters: ZSTD_sequenceFormat_e,
        validate_sequences: bool,
        enable_seq_producer_fallback: bool,
        dict_content_type: ZSTD_dictContentType_e,
        static_workspace_size: usize,
    ) -> Result<Self, ZSTD_ErrorCode> {
        validate_dictionary_source(bytes, dict_content_type)?;
        let dict_id = match dict_content_type {
            ZSTD_dictContentType_e::ZSTD_dct_rawContent => 0,
            ZSTD_dictContentType_e::ZSTD_dct_fullDict => {
                ddict::ZSTD_getDictID_fromDict(bytes.as_ptr().cast(), bytes.len())
            }
            ZSTD_dictContentType_e::ZSTD_dct_auto => {
                if is_formatted_dictionary(bytes) {
                    ddict::ZSTD_getDictID_fromDict(bytes.as_ptr().cast(), bytes.len())
                } else {
                    0
                }
            }
        };
        Ok(Self {
            storage,
            static_workspace_size,
            dict_id,
            compression_level,
            cparams: normalize_cparams(cparams),
            enable_long_distance_matching,
            nb_workers,
            job_size,
            overlap_log,
            block_delimiters,
            validate_sequences,
            enable_seq_producer_fallback,
            dict_content_type,
        })
    }

    fn from_bytes(bytes: &[u8], compression_level: c_int) -> Result<Self, ZSTD_ErrorCode> {
        let cparams = get_cparams(compression_level, ZSTD_CONTENTSIZE_UNKNOWN, bytes.len());
        Self::from_settings(
            bytes,
            compression_level,
            cparams,
            false,
            0,
            0,
            0,
            ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters,
            false,
            false,
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy,
            ZSTD_dictContentType_e::ZSTD_dct_auto,
        )
    }

    fn from_settings(
        bytes: &[u8],
        compression_level: c_int,
        cparams: ZSTD_compressionParameters,
        enable_long_distance_matching: bool,
        nb_workers: c_int,
        job_size: c_int,
        overlap_log: c_int,
        block_delimiters: ZSTD_sequenceFormat_e,
        validate_sequences: bool,
        enable_seq_producer_fallback: bool,
        dict_load_method: ZSTD_dictLoadMethod_e,
        dict_content_type: ZSTD_dictContentType_e,
    ) -> Result<Self, ZSTD_ErrorCode> {
        Self::from_storage(
            EncoderDictionaryStorage::from_load_method(bytes, dict_load_method),
            bytes,
            compression_level,
            cparams,
            enable_long_distance_matching,
            nb_workers,
            job_size,
            overlap_log,
            block_delimiters,
            validate_sequences,
            enable_seq_producer_fallback,
            dict_content_type,
            0,
        )
    }

    fn bytes(&self) -> &[u8] {
        self.storage.as_slice()
    }

    fn len(&self) -> usize {
        self.bytes().len()
    }

    fn heap_size(&self) -> usize {
        self.storage.heap_size()
    }

    fn workspace_size(&self) -> usize {
        self.static_workspace_size
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StreamState {
    pub(crate) input: Vec<u8>,
    pub(crate) pending: Vec<u8>,
    pub(crate) pending_pos: usize,
    pub(crate) emitted_input: usize,
    pub(crate) produced_total: usize,
    pub(crate) flushed_total: usize,
    pub(crate) frame_started: bool,
    pub(crate) frame_finished: bool,
    pub(crate) deferred_header: bool,
}

impl StreamState {
    pub(crate) fn reset(&mut self) {
        self.input.clear();
        self.pending.clear();
        self.pending_pos = 0;
        self.emitted_input = 0;
        self.produced_total = 0;
        self.flushed_total = 0;
        self.frame_started = false;
        self.frame_finished = false;
        self.deferred_header = false;
    }

    pub(crate) fn size_of(&self) -> usize {
        alloc::heap_bytes(self.input.len() + self.pending.len())
    }
}

struct DictionaryMatcher {
    history_seed: Vec<u8>,
    history: Vec<u8>,
    last_space: Vec<u8>,
    finder: OxiarcMatchFinder,
    slice_size: usize,
    max_window_size: usize,
    match_level: i32,
}

impl DictionaryMatcher {
    fn new(
        history_seed: Vec<u8>,
        slice_size: usize,
        max_window_size: usize,
        match_level: i32,
    ) -> Self {
        let slice_size = slice_size.max(1);
        let mut config = OxiarcLevelConfig::for_level(match_level.max(1));
        config.target_block_size = slice_size.min(ZSTD_BLOCKSIZE_MAX).max(1);
        Self {
            history_seed,
            history: Vec::new(),
            last_space: Vec::new(),
            finder: OxiarcMatchFinder::new(&config),
            slice_size,
            max_window_size: max_window_size.max(slice_size),
            match_level,
        }
    }

    fn reset_state(&mut self) {
        let mut config = OxiarcLevelConfig::for_level(self.match_level.max(1));
        config.target_block_size = self.slice_size.min(ZSTD_BLOCKSIZE_MAX).max(1);
        self.finder = OxiarcMatchFinder::new(&config);
        self.history.clear();
        if !self.history_seed.is_empty() {
            self.history.extend_from_slice(&self.history_seed);
            if self.history.len() > self.max_window_size {
                let trim = self.history.len() - self.max_window_size;
                self.history.drain(..trim);
            }
        }
        self.last_space.clear();
    }

    fn remember_last_space(&mut self) {
        if self.last_space.is_empty() {
            return;
        }
        self.history.extend_from_slice(&self.last_space);
        if self.history.len() > self.max_window_size {
            let trim = self.history.len() - self.max_window_size;
            self.history.drain(..trim);
        }
        self.last_space.clear();
    }
}

impl Matcher for DictionaryMatcher {
    fn get_next_space(&mut self) -> Vec<u8> {
        vec![0u8; self.slice_size]
    }

    fn get_last_space(&mut self) -> &[u8] {
        self.last_space.as_slice()
    }

    fn commit_space(&mut self, space: Vec<u8>) {
        self.last_space = space;
    }

    fn skip_matching(&mut self) {
        self.remember_last_space();
    }

    fn start_matching(&mut self, mut handle_sequence: impl for<'a> FnMut(StructuredSequence<'a>)) {
        let sequences = self
            .finder
            .find_sequences(self.last_space.as_slice(), self.history.as_slice())
            .unwrap_or_default();
        for sequence in &sequences {
            if sequence.match_length == 0 {
                if !sequence.literals.is_empty() {
                    handle_sequence(StructuredSequence::Literals {
                        literals: sequence.literals.as_slice(),
                    });
                }
            } else {
                handle_sequence(StructuredSequence::Triple {
                    literals: sequence.literals.as_slice(),
                    offset: sequence.offset,
                    match_len: sequence.match_length,
                });
            }
        }
        self.finder.reset();
        self.remember_last_space();
    }

    fn reset(&mut self, _level: StructuredCompressionLevel) {
        self.reset_state();
    }

    fn window_size(&self) -> u64 {
        self.max_window_size as u64
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EncoderContext {
    pub(crate) static_workspace_size: usize,
    pub compression_level: c_int,
    pub cparams: ZSTD_compressionParameters,
    pub fparams: ZSTD_frameParameters,
    pub dict: Option<EncoderDictionary>,
    pub prefix: Option<Vec<u8>>,
    pub prefix_content_type: ZSTD_dictContentType_e,
    pub pledged_src_size: u64,
    pub nb_workers: c_int,
    pub job_size: c_int,
    pub overlap_log: c_int,
    pub block_delimiters: ZSTD_sequenceFormat_e,
    pub enable_long_distance_matching: bool,
    pub validate_sequences: bool,
    pub enable_seq_producer_fallback: bool,
    pub sequence_producer_state: *mut c_void,
    pub sequence_producer: Option<ZSTD_sequenceProducer_F>,
    pub thread_pool: *mut ZSTD_threadPool,
    pub stream_mode: bool,
    pub legacy_mode: bool,
    pub(crate) block_history: Vec<u8>,
    pub(crate) stream: StreamState,
    pub(crate) legacy_input: Vec<u8>,
}

impl Default for EncoderContext {
    fn default() -> Self {
        Self {
            static_workspace_size: 0,
            compression_level: ZSTD_CLEVEL_DEFAULT,
            cparams: default_cparams(),
            fparams: default_params().fParams,
            dict: None,
            prefix: None,
            prefix_content_type: ZSTD_dictContentType_e::ZSTD_dct_auto,
            pledged_src_size: ZSTD_CONTENTSIZE_UNKNOWN,
            nb_workers: 0,
            job_size: 0,
            overlap_log: 0,
            block_delimiters: ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters,
            enable_long_distance_matching: false,
            validate_sequences: false,
            enable_seq_producer_fallback: false,
            sequence_producer_state: core::ptr::null_mut(),
            sequence_producer: None,
            thread_pool: core::ptr::null_mut(),
            stream_mode: false,
            legacy_mode: false,
            block_history: Vec::new(),
            stream: StreamState::default(),
            legacy_input: Vec::new(),
        }
    }
}

impl EncoderContext {
    pub(crate) fn mark_static(&mut self, workspace_size: usize) {
        self.static_workspace_size = workspace_size;
    }

    pub(crate) fn sizeof(&self) -> usize {
        self.static_workspace_size.max(alloc::base_size::<Self>())
            + self.stream.size_of()
            + alloc::heap_bytes(self.legacy_input.len())
            + alloc::heap_bytes(self.block_history.len())
            + self.dict.as_ref().map_or(0, EncoderDictionary::heap_size)
            + self
                .prefix
                .as_ref()
                .map_or(0, |prefix| alloc::heap_bytes(prefix.len()))
    }

    pub(crate) fn clear_session(&mut self) {
        self.pledged_src_size = ZSTD_CONTENTSIZE_UNKNOWN;
        self.stream_mode = false;
        self.legacy_mode = false;
        self.block_history.clear();
        self.stream.reset();
        self.legacy_input.clear();
    }

    pub(crate) fn clear_parameters(&mut self) {
        self.compression_level = ZSTD_CLEVEL_DEFAULT;
        self.cparams = default_cparams();
        self.fparams = default_params().fParams;
        self.dict = None;
        self.prefix = None;
        self.prefix_content_type = ZSTD_dictContentType_e::ZSTD_dct_auto;
        self.nb_workers = 0;
        self.job_size = 0;
        self.overlap_log = 0;
        self.block_delimiters = ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters;
        self.enable_long_distance_matching = false;
        self.validate_sequences = false;
        self.enable_seq_producer_fallback = false;
        self.sequence_producer_state = core::ptr::null_mut();
        self.sequence_producer = None;
        self.thread_pool = core::ptr::null_mut();
    }

    pub(crate) fn reset(&mut self, reset: ZSTD_ResetDirective) {
        match reset {
            ZSTD_ResetDirective::ZSTD_reset_session_only => self.clear_session(),
            ZSTD_ResetDirective::ZSTD_reset_parameters => self.clear_parameters(),
            ZSTD_ResetDirective::ZSTD_reset_session_and_parameters => {
                self.clear_session();
                self.clear_parameters();
            }
        }
    }

    pub(crate) fn set_dict(&mut self, dict: Option<EncoderDictionary>) {
        self.dict = dict;
        self.prefix = None;
        self.prefix_content_type = ZSTD_dictContentType_e::ZSTD_dct_auto;
    }

    pub(crate) fn apply_cdict(&mut self, dict: EncoderDictionary) {
        self.compression_level = dict.compression_level;
        self.cparams = dict.cparams;
        self.nb_workers = dict.nb_workers;
        self.job_size = dict.job_size;
        self.overlap_log = dict.overlap_log;
        self.block_delimiters = dict.block_delimiters;
        self.enable_long_distance_matching = dict.enable_long_distance_matching;
        self.validate_sequences = dict.validate_sequences;
        self.enable_seq_producer_fallback = dict.enable_seq_producer_fallback;
        self.set_dict(Some(dict));
    }

    pub(crate) fn set_prefix(
        &mut self,
        prefix: Option<&[u8]>,
        dict_content_type: ZSTD_dictContentType_e,
    ) {
        self.dict = None;
        self.prefix = prefix.filter(|bytes| !bytes.is_empty()).map(Vec::from);
        self.prefix_content_type = if self.prefix.is_some() {
            dict_content_type
        } else {
            ZSTD_dictContentType_e::ZSTD_dct_auto
        };
    }

    pub(crate) fn apply_params(&mut self, params: ZSTD_parameters) {
        self.cparams = normalize_cparams(params.cParams);
        self.fparams = params.fParams;
    }

    pub(crate) fn dict_id_for_frame(&self) -> u32 {
        if self.fparams.noDictIDFlag != 0 {
            0
        } else {
            self.dict.as_ref().map_or(0, |dict| dict.dict_id)
        }
    }

    pub(crate) fn window_size(&self) -> usize {
        1usize << self.cparams.windowLog.min(30)
    }

    pub(crate) fn frame_block_size(&self) -> usize {
        self.window_size().min(ZSTD_BLOCKSIZE_MAX)
    }

    pub(crate) fn push_block_history(&mut self, src: &[u8]) {
        if src.is_empty() {
            return;
        }
        self.block_history.extend_from_slice(src);
        let limit = self.window_size().max(1);
        if self.block_history.len() > limit {
            let trim = self.block_history.len() - limit;
            self.block_history.drain(..trim);
        }
    }

    pub(crate) fn check_pledged_size(&self, src_size: usize) -> Result<(), ZSTD_ErrorCode> {
        if self.pledged_src_size != ZSTD_CONTENTSIZE_UNKNOWN
            && self.pledged_src_size != src_size as u64
        {
            return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
        }
        Ok(())
    }
}

pub(crate) fn null_cctx() -> *mut ZSTD_CCtx {
    core::ptr::null_mut()
}

pub(crate) fn null_cdict() -> *mut ZSTD_CDict {
    core::ptr::null_mut()
}

pub(crate) fn default_cparams() -> ZSTD_compressionParameters {
    ZSTD_compressionParameters {
        windowLog: 19,
        chainLog: 18,
        hashLog: 18,
        searchLog: 1,
        minMatch: 4,
        targetLength: 0,
        strategy: ZSTD_strategy::ZSTD_fast,
    }
}

fn negative_level_cparams(src_size_hint: u64, target_length: u32) -> ZSTD_compressionParameters {
    let window_log = if src_size_hint == ZSTD_CONTENTSIZE_UNKNOWN {
        19
    } else {
        let needed = 64 - src_size_hint.max(1).leading_zeros();
        needed.clamp(10, 27)
    };
    ZSTD_compressionParameters {
        windowLog: window_log,
        chainLog: window_log.saturating_sub(7),
        hashLog: window_log.saturating_sub(6),
        searchLog: 1,
        minMatch: 6,
        targetLength: target_length,
        strategy: ZSTD_strategy::ZSTD_fast,
    }
}

pub(crate) fn min_clevel() -> c_int {
    ZSTD_MIN_CLEVEL
}

fn normalize_src_size_hint(src_size_hint: u64) -> u64 {
    if src_size_hint == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        src_size_hint
    }
}

fn normalize_compression_level(compression_level: c_int) -> c_int {
    if compression_level == 0 {
        ZSTD_CLEVEL_DEFAULT
    } else {
        compression_level.clamp(ZSTD_MIN_CLEVEL, ZSTD_MAX_CLEVEL)
    }
}

pub(crate) fn default_params() -> ZSTD_parameters {
    ZSTD_parameters {
        cParams: default_cparams(),
        fParams: ZSTD_frameParameters {
            contentSizeFlag: 1,
            checksumFlag: 0,
            noDictIDFlag: 0,
        },
    }
}

pub(crate) fn optional_src_slice<'a>(ptr: *const c_void, len: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return (len == 0).then_some(&[]);
    }
    Some(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) })
}

pub(crate) fn optional_src_slice_mut<'a>(ptr: *mut c_void, len: usize) -> Option<&'a mut [u8]> {
    if ptr.is_null() {
        return (len == 0).then_some(&mut []);
    }
    Some(unsafe { core::slice::from_raw_parts_mut(ptr.cast::<u8>(), len) })
}

fn cctx_ref<'a>(ptr: *const ZSTD_CCtx) -> Option<&'a EncoderContext> {
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &*ptr.cast::<EncoderContext>() })
}

fn cctx_mut<'a>(ptr: *mut ZSTD_CCtx) -> Option<&'a mut EncoderContext> {
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &mut *ptr.cast::<EncoderContext>() })
}

fn cdict_ref<'a>(ptr: *const ZSTD_CDict) -> Option<&'a EncoderDictionary> {
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &*ptr.cast::<EncoderDictionary>() })
}

pub(crate) fn with_cctx_ref<T>(
    ptr: *const ZSTD_CCtx,
    f: impl FnOnce(&EncoderContext) -> Result<T, ZSTD_ErrorCode>,
) -> Result<T, ZSTD_ErrorCode> {
    let cctx = cctx_ref(ptr).ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    f(cctx)
}

pub(crate) fn with_cctx_mut<T>(
    ptr: *mut ZSTD_CCtx,
    f: impl FnOnce(&mut EncoderContext) -> Result<T, ZSTD_ErrorCode>,
) -> Result<T, ZSTD_ErrorCode> {
    let cctx = cctx_mut(ptr).ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    f(cctx)
}

pub(crate) fn with_cdict_ref<T>(
    ptr: *const ZSTD_CDict,
    f: impl FnOnce(&EncoderDictionary) -> Result<T, ZSTD_ErrorCode>,
) -> Result<T, ZSTD_ErrorCode> {
    let cdict = cdict_ref(ptr).ok_or(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong)?;
    f(cdict)
}

pub(crate) fn create_cctx() -> *mut ZSTD_CCtx {
    Box::into_raw(Box::new(EncoderContext::default())).cast()
}

pub(crate) fn init_static_cctx(workspace: *mut c_void, workspace_size: usize) -> *mut ZSTD_CCtx {
    if workspace.is_null()
        || workspace_size <= size_of::<EncoderContext>()
        || (workspace as usize & 7) != 0
    {
        return null_cctx();
    }
    let ptr = workspace.cast::<EncoderContext>();
    unsafe {
        ptr.write(EncoderContext::default());
        (*ptr).mark_static(workspace_size);
    }
    ptr.cast()
}

pub(crate) fn free_cctx(ptr: *mut ZSTD_CCtx) -> usize {
    if ptr.is_null() {
        return 0;
    }
    if cctx_ref(ptr.cast_const()).is_some_and(|cctx| cctx.static_workspace_size != 0) {
        return error_result(ZSTD_ErrorCode::ZSTD_error_memory_allocation);
    }
    unsafe {
        drop(Box::from_raw(ptr.cast::<EncoderContext>()));
    }
    0
}

pub(crate) fn sizeof_cctx(ptr: *const ZSTD_CCtx) -> usize {
    cctx_ref(ptr).map_or(0, EncoderContext::sizeof)
}

fn validate_dictionary_source(
    dict: &[u8],
    dict_content_type: ZSTD_dictContentType_e,
) -> Result<(), ZSTD_ErrorCode> {
    crate::decompress::fse::validate_dictionary_kind(dict)?;
    match dict_content_type {
        ZSTD_dictContentType_e::ZSTD_dct_fullDict => {
            if !is_formatted_dictionary(dict) {
                return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
            }
            formatted_dict_content(dict).map(|_| ())
        }
        ZSTD_dictContentType_e::ZSTD_dct_auto if is_formatted_dictionary(dict) => {
            formatted_dict_content(dict).map(|_| ())
        }
        _ => Ok(()),
    }
}

pub(crate) fn create_cdict(dict: &[u8], compression_level: c_int) -> *mut ZSTD_CDict {
    match EncoderDictionary::from_bytes(dict, compression_level) {
        Ok(dict) => Box::into_raw(Box::new(dict)).cast(),
        Err(_) => null_cdict(),
    }
}

pub(crate) fn create_cdict_with_settings(
    dict: &[u8],
    compression_level: c_int,
    cparams: ZSTD_compressionParameters,
    enable_long_distance_matching: bool,
    nb_workers: c_int,
    job_size: c_int,
    overlap_log: c_int,
    block_delimiters: ZSTD_sequenceFormat_e,
    validate_sequences: bool,
    enable_seq_producer_fallback: bool,
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
) -> *mut ZSTD_CDict {
    match EncoderDictionary::from_settings(
        dict,
        compression_level,
        cparams,
        enable_long_distance_matching,
        nb_workers,
        job_size,
        overlap_log,
        block_delimiters,
        validate_sequences,
        enable_seq_producer_fallback,
        dict_load_method,
        dict_content_type,
    ) {
        Ok(dict) => Box::into_raw(Box::new(dict)).cast(),
        Err(_) => null_cdict(),
    }
}

pub(crate) fn init_static_cdict(
    workspace: *mut c_void,
    workspace_size: usize,
    dict: &[u8],
    compression_level: c_int,
    cparams: ZSTD_compressionParameters,
    enable_long_distance_matching: bool,
    nb_workers: c_int,
    job_size: c_int,
    overlap_log: c_int,
    block_delimiters: ZSTD_sequenceFormat_e,
    validate_sequences: bool,
    enable_seq_producer_fallback: bool,
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
) -> *mut ZSTD_CDict {
    if workspace.is_null()
        || workspace_size < cdict_size_estimate_advanced(dict.len(), dict_load_method)
        || (workspace as usize & 7) != 0
    {
        return null_cdict();
    }

    let storage = match dict_load_method {
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy => {
            let dict_ptr = unsafe { workspace.cast::<u8>().add(size_of::<EncoderDictionary>()) };
            unsafe {
                core::ptr::copy_nonoverlapping(dict.as_ptr(), dict_ptr, dict.len());
            }
            EncoderDictionaryStorage::Referenced(dict_ptr, dict.len())
        }
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef => {
            EncoderDictionaryStorage::Referenced(dict.as_ptr(), dict.len())
        }
    };

    let Ok(cdict) = EncoderDictionary::from_storage(
        storage,
        dict,
        compression_level,
        cparams,
        enable_long_distance_matching,
        nb_workers,
        job_size,
        overlap_log,
        block_delimiters,
        validate_sequences,
        enable_seq_producer_fallback,
        dict_content_type,
        workspace_size,
    ) else {
        return null_cdict();
    };

    let ptr = workspace.cast::<EncoderDictionary>();
    unsafe {
        ptr.write(cdict);
    }
    ptr.cast()
}

pub(crate) fn free_cdict(ptr: *mut ZSTD_CDict) -> usize {
    if ptr.is_null() {
        return 0;
    }
    if cdict_ref(ptr.cast_const()).is_some_and(|cdict| cdict.workspace_size() != 0) {
        return 0;
    }
    unsafe {
        drop(Box::from_raw(ptr.cast::<EncoderDictionary>()));
    }
    0
}

pub(crate) fn sizeof_cdict(ptr: *const ZSTD_CDict) -> usize {
    cdict_ref(ptr).map_or(0, |cdict| {
        cdict
            .workspace_size()
            .max(alloc::base_size::<EncoderDictionary>())
            + cdict.heap_size()
    })
}

pub(crate) fn compress_bound(src_size: usize) -> usize {
    const MIN_BLOCK_SIZE: usize = 1 << 10;
    let blocks = if src_size == 0 {
        1
    } else {
        src_size.div_ceil(MIN_BLOCK_SIZE)
    };
    src_size
        .saturating_add(blocks.saturating_mul(8))
        .saturating_add(32)
}

fn is_all_same(src: &[u8]) -> bool {
    src.first()
        .map(|first| src.iter().all(|byte| *byte == *first))
        .unwrap_or(true)
}

fn write_block_header(dst: &mut Vec<u8>, last_block: bool, block_type: u8, content_size: usize) {
    let value = usize::from(last_block) | ((block_type as usize) << 1) | (content_size << 3);
    dst.extend_from_slice(&(value as u32).to_le_bytes()[..BLOCK_HEADER_SIZE]);
}

fn xxh64_round(acc: u64, input: u64) -> u64 {
    let acc = acc.wrapping_add(input.wrapping_mul(XXH64_PRIME_2));
    acc.rotate_left(31).wrapping_mul(XXH64_PRIME_1)
}

fn xxh64_merge_round(acc: u64, value: u64) -> u64 {
    let acc = acc ^ xxh64_round(0, value);
    acc.wrapping_mul(XXH64_PRIME_1).wrapping_add(XXH64_PRIME_4)
}

fn read_u32(input: &[u8]) -> u32 {
    u32::from_le_bytes(input[..4].try_into().expect("slice length checked"))
}

fn read_u64(input: &[u8]) -> u64 {
    u64::from_le_bytes(input[..8].try_into().expect("slice length checked"))
}

pub(crate) fn xxh64(src: &[u8]) -> u64 {
    let mut offset = 0usize;
    let mut hash = if src.len() >= 32 {
        let mut v1 = XXH64_SEED
            .wrapping_add(XXH64_PRIME_1)
            .wrapping_add(XXH64_PRIME_2);
        let mut v2 = XXH64_SEED.wrapping_add(XXH64_PRIME_2);
        let mut v3 = XXH64_SEED;
        let mut v4 = XXH64_SEED.wrapping_sub(XXH64_PRIME_1);

        while offset + 32 <= src.len() {
            v1 = xxh64_round(v1, read_u64(&src[offset..]));
            offset += 8;
            v2 = xxh64_round(v2, read_u64(&src[offset..]));
            offset += 8;
            v3 = xxh64_round(v3, read_u64(&src[offset..]));
            offset += 8;
            v4 = xxh64_round(v4, read_u64(&src[offset..]));
            offset += 8;
        }

        let mut hash = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        hash = xxh64_merge_round(hash, v1);
        hash = xxh64_merge_round(hash, v2);
        hash = xxh64_merge_round(hash, v3);
        xxh64_merge_round(hash, v4)
    } else {
        XXH64_SEED.wrapping_add(XXH64_PRIME_5)
    };

    hash = hash.wrapping_add(src.len() as u64);

    while offset + 8 <= src.len() {
        let lane = xxh64_round(0, read_u64(&src[offset..]));
        hash ^= lane;
        hash = hash
            .rotate_left(27)
            .wrapping_mul(XXH64_PRIME_1)
            .wrapping_add(XXH64_PRIME_4);
        offset += 8;
    }

    if offset + 4 <= src.len() {
        hash ^= u64::from(read_u32(&src[offset..])).wrapping_mul(XXH64_PRIME_1);
        hash = hash
            .rotate_left(23)
            .wrapping_mul(XXH64_PRIME_2)
            .wrapping_add(XXH64_PRIME_3);
        offset += 4;
    }

    while offset < src.len() {
        hash ^= u64::from(src[offset]).wrapping_mul(XXH64_PRIME_5);
        hash = hash.rotate_left(11).wrapping_mul(XXH64_PRIME_1);
        offset += 1;
    }

    hash ^= hash >> 33;
    hash = hash.wrapping_mul(XXH64_PRIME_2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(XXH64_PRIME_3);
    hash ^ (hash >> 32)
}

fn encode_window_descriptor(window_size: u64) -> Result<u8, ZSTD_ErrorCode> {
    let window_size = window_size.max(1u64 << 10);
    for window_log in 10u64..=31 {
        let base = 1u64 << window_log;
        if window_size < base {
            continue;
        }
        let step = base >> 3;
        let diff = window_size - base;
        if diff > step * 7 || diff % step != 0 {
            continue;
        }
        return Ok((((window_log - 10) as u8) << 3) | (diff / step) as u8);
    }
    Err(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)
}

fn dict_id_code(dict_id: u32) -> u8 {
    if dict_id == 0 {
        0
    } else if dict_id <= u8::MAX as u32 {
        1
    } else if dict_id <= u16::MAX as u32 {
        2
    } else {
        3
    }
}

fn fcs_bytes(src_size: usize) -> (u8, Vec<u8>) {
    if src_size <= u8::MAX as usize {
        (0, vec![src_size as u8])
    } else if u32::try_from(src_size).is_ok() {
        (2, (src_size as u32).to_le_bytes().to_vec())
    } else {
        (3, (src_size as u64).to_le_bytes().to_vec())
    }
}

fn required_window_size(required: usize) -> u64 {
    let required = required.max(1usize << 10);
    required
        .checked_next_power_of_two()
        .unwrap_or(1usize << 31)
        .min(1usize << 31) as u64
}

fn build_frame_header(
    ctx: &EncoderContext,
    content_size: Option<usize>,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let mut header = Vec::with_capacity(16);
    header.extend_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());

    let dict_id = ctx.dict_id_for_frame();
    let dict_code = dict_id_code(dict_id);
    let checksum_flag = u8::from(ctx.fparams.checksumFlag != 0);
    let content_size_flag = ctx.fparams.contentSizeFlag != 0;
    let external_history_len = compression_history(ctx)?.map_or(0usize, |history| history.len());
    let window_requirement = if let Some(src_size) = content_size {
        external_history_len.saturating_add(src_size)
    } else {
        external_history_len
            .saturating_add(ctx.stream.input.len().max(ctx.frame_block_size()))
            .max(ctx.frame_block_size())
    };
    let mut window_size = required_window_size(window_requirement);
    if content_size.is_none() {
        window_size = window_size.max(1u64 << default_cparams().windowLog.min(31));
    }
    window_size = window_size.max(1u64 << ctx.cparams.windowLog.min(31));
    let can_use_single_segment =
        content_size.is_some_and(|src_size| external_history_len <= src_size);
    if content_size_flag {
        if let Some(src_size) = content_size {
            if can_use_single_segment {
                let (fcs_code, fcs) = fcs_bytes(src_size);
                let descriptor = dict_code | (checksum_flag << 2) | (1 << 5) | (fcs_code << 6);
                header.push(descriptor);
                match dict_code {
                    0 => {}
                    1 => header.push(dict_id as u8),
                    2 => header.extend_from_slice(&(dict_id as u16).to_le_bytes()),
                    3 => header.extend_from_slice(&dict_id.to_le_bytes()),
                    _ => unreachable!(),
                }
                header.extend_from_slice(&fcs);
            } else {
                let (fcs_code, fcs) = fcs_bytes(src_size);
                let descriptor = dict_code | (checksum_flag << 2) | (fcs_code << 6);
                header.push(descriptor);
                header.push(encode_window_descriptor(window_size)?);
                match dict_code {
                    0 => {}
                    1 => header.push(dict_id as u8),
                    2 => header.extend_from_slice(&(dict_id as u16).to_le_bytes()),
                    3 => header.extend_from_slice(&dict_id.to_le_bytes()),
                    _ => unreachable!(),
                }
                header.extend_from_slice(&fcs);
            }
        } else {
            let descriptor = dict_code | (checksum_flag << 2);
            header.push(descriptor);
            header.push(encode_window_descriptor(window_size)?);
            match dict_code {
                0 => {}
                1 => header.push(dict_id as u8),
                2 => header.extend_from_slice(&(dict_id as u16).to_le_bytes()),
                3 => header.extend_from_slice(&dict_id.to_le_bytes()),
                _ => unreachable!(),
            }
        }
    } else {
        let descriptor = dict_code | (checksum_flag << 2);
        header.push(descriptor);
        header.push(encode_window_descriptor(window_size)?);
        match dict_code {
            0 => {}
            1 => header.push(dict_id as u8),
            2 => header.extend_from_slice(&(dict_id as u16).to_le_bytes()),
            3 => header.extend_from_slice(&dict_id.to_le_bytes()),
            _ => unreachable!(),
        }
    }

    Ok(header)
}

fn append_pending(stream: &mut StreamState, bytes: &[u8]) {
    if bytes.is_empty() {
        if stream.pending_pos == stream.pending.len() {
            stream.pending.clear();
            stream.pending_pos = 0;
        }
        return;
    }

    if stream.pending_pos == stream.pending.len() {
        stream.pending.clear();
        stream.pending_pos = 0;
    } else if stream.pending_pos > 0 && stream.pending_pos * 2 >= stream.pending.len() {
        stream.pending.drain(..stream.pending_pos);
        stream.pending_pos = 0;
    }

    stream.pending.extend_from_slice(bytes);
    stream.produced_total = stream.produced_total.saturating_add(bytes.len());
}

fn append_pending_stored_blocks(
    stream: &mut StreamState,
    src: &[u8],
    block_size: usize,
    last_block: bool,
) {
    let start = stream.pending.len();
    append_stored_blocks(&mut stream.pending, src, block_size, last_block);
    stream.produced_total = stream
        .produced_total
        .saturating_add(stream.pending.len().saturating_sub(start));
}

fn effective_oxiarc_level(level: c_int) -> i32 {
    let level = normalize_compression_level(level);
    if level < 1 {
        ZSTD_CLEVEL_DEFAULT.max(1)
    } else {
        level
    }
}

fn formatted_dict_content(bytes: &[u8]) -> Result<&[u8], ZSTD_ErrorCode> {
    crate::dict_builder::zdict::formatted_dictionary_content(bytes)
}

fn dictionary_history(bytes: &[u8]) -> Result<Cow<'_, [u8]>, ZSTD_ErrorCode> {
    if bytes.is_empty() {
        return Ok(Cow::Borrowed(bytes));
    }
    if is_formatted_dictionary(bytes) {
        if let Ok(dict) = StructuredDictionary::decode_dict(bytes) {
            return Ok(Cow::Owned(dict.dict_content));
        }
        return formatted_dict_content(bytes).map(Cow::Borrowed);
    } else {
        return Ok(Cow::Borrowed(bytes));
    }
}

fn prefix_history(
    bytes: &[u8],
    dict_content_type: ZSTD_dictContentType_e,
) -> Result<Cow<'_, [u8]>, ZSTD_ErrorCode> {
    match dict_content_type {
        ZSTD_dictContentType_e::ZSTD_dct_fullDict => {
            if !is_formatted_dictionary(bytes) {
                return Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
            }
            dictionary_history(bytes)
        }
        ZSTD_dictContentType_e::ZSTD_dct_auto if is_formatted_dictionary(bytes) => {
            dictionary_history(bytes)
        }
        _ => Ok(Cow::Borrowed(bytes)),
    }
}

fn ignore_dictionary_history(bytes: &[u8], dict_content_type: ZSTD_dictContentType_e) -> bool {
    let formatted_history_supported = formatted_dict_content(bytes).is_ok();
    match dict_content_type {
        ZSTD_dictContentType_e::ZSTD_dct_fullDict => !formatted_history_supported,
        ZSTD_dictContentType_e::ZSTD_dct_auto => {
            is_formatted_dictionary(bytes) && !formatted_history_supported
        }
        ZSTD_dictContentType_e::ZSTD_dct_rawContent => false,
    }
}

fn compression_history(ctx: &EncoderContext) -> Result<Option<Cow<'_, [u8]>>, ZSTD_ErrorCode> {
    if let Some(prefix) = ctx.prefix.as_deref() {
        if ignore_dictionary_history(prefix, ctx.prefix_content_type) {
            return Ok(None);
        }
        return prefix_history(prefix, ctx.prefix_content_type).map(Some);
    }
    if let Some(dict) = ctx.dict.as_ref() {
        if ignore_dictionary_history(dict.bytes(), dict.dict_content_type) {
            return Ok(None);
        }
        return prefix_history(dict.bytes(), dict.dict_content_type).map(Some);
    }
    Ok(None)
}

fn trim_history(history: &mut Vec<u8>, limit: usize) {
    if history.len() > limit {
        let trim = history.len() - limit;
        history.drain(..trim);
    }
}

fn block_history_seed(ctx: &EncoderContext) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let limit = ctx.window_size().max(1);
    let mut history = compression_history(ctx)?
        .map(Cow::into_owned)
        .unwrap_or_default();
    trim_history(&mut history, limit);
    if !ctx.block_history.is_empty() {
        history.extend_from_slice(&ctx.block_history);
        trim_history(&mut history, limit);
    }
    Ok(history)
}

fn block_body_bounds(payload: &[u8]) -> Result<(u8, &[u8]), ZSTD_ErrorCode> {
    if payload.len() < BLOCK_HEADER_SIZE {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }
    let header =
        u32::from(payload[0]) | (u32::from(payload[1]) << 8) | (u32::from(payload[2]) << 16);
    let block_type = ((header >> 1) & 0x3) as u8;
    let size = (header >> 3) as usize;
    let body_len = match block_type {
        0 | 2 => size,
        1 => usize::from(size != 0),
        _ => return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC),
    };
    let end = BLOCK_HEADER_SIZE
        .checked_add(body_len)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    if end > payload.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }
    Ok((block_type, &payload[BLOCK_HEADER_SIZE..end]))
}

fn last_block_header_offset(payload: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    let mut offset = 0usize;

    loop {
        let header_end = offset
            .checked_add(BLOCK_HEADER_SIZE)
            .ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
        if header_end > payload.len() {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        }

        let header = u32::from(payload[offset])
            | (u32::from(payload[offset + 1]) << 8)
            | (u32::from(payload[offset + 2]) << 16);
        let last_block = (header & 1) != 0;
        let block_type = ((header >> 1) & 0x3) as u8;
        let size = (header >> 3) as usize;
        let body_len = match block_type {
            0 | 2 => size,
            1 => usize::from(size != 0),
            _ => return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC),
        };
        let end = header_end
            .checked_add(body_len)
            .ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
        if end > payload.len() {
            return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
        }
        if last_block {
            return Ok(offset);
        }
        offset = end;
    }
}

fn structured_level(level: c_int) -> StructuredCompressionLevel {
    if normalize_compression_level(level) >= 4 {
        StructuredCompressionLevel::Default
    } else {
        StructuredCompressionLevel::Fastest
    }
}

fn structured_payload(
    history: &[u8],
    src: &[u8],
    ctx: &EncoderContext,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let slice_size = ctx.frame_block_size().max(1);
    let history_limit = ctx.window_size().max(1);
    let history = if history.len() > history_limit {
        history[history.len() - history_limit..].to_vec()
    } else {
        history.to_vec()
    };
    let max_window_size = history.len().saturating_add(slice_size).max(slice_size);
    let matcher = DictionaryMatcher::new(
        history,
        slice_size,
        max_window_size,
        normalize_compression_level(ctx.compression_level).max(1),
    );
    let mut compressor =
        FrameCompressor::new_with_matcher(matcher, structured_level(ctx.compression_level));
    let mut encoded = Vec::with_capacity(src.len().saturating_add(32));
    compressor.set_source(src);
    compressor.set_drain(&mut encoded);
    compressor.compress();

    let header = match parse_frame_header(&encoded, ZSTD_format_e::ZSTD_f_zstd1)? {
        HeaderProbe::Header(header) => header,
        HeaderProbe::Need(_) => return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC),
    };
    let start = header.headerSize as usize;
    let trailer = usize::from(header.checksumFlag != 0) * 4;
    if start > encoded.len() || start + trailer > encoded.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }
    Ok(encoded[start..encoded.len() - trailer].to_vec())
}

fn uses_formatted_dictionary_source(
    bytes: &[u8],
    dict_content_type: ZSTD_dictContentType_e,
) -> bool {
    match dict_content_type {
        ZSTD_dictContentType_e::ZSTD_dct_fullDict => !bytes.is_empty(),
        ZSTD_dictContentType_e::ZSTD_dct_auto => !bytes.is_empty(),
        ZSTD_dictContentType_e::ZSTD_dct_rawContent => !bytes.is_empty(),
    }
}

fn uses_formatted_dictionary_history(ctx: &EncoderContext) -> bool {
    if let Some(prefix) = ctx.prefix.as_deref() {
        return uses_formatted_dictionary_source(prefix, ctx.prefix_content_type);
    }
    if let Some(dict) = ctx.dict.as_ref() {
        return uses_formatted_dictionary_source(dict.bytes(), dict.dict_content_type);
    }
    false
}

fn payload_with_history(
    history: &[u8],
    src: &[u8],
    ctx: &EncoderContext,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    if uses_formatted_dictionary_history(ctx) {
        return structured_payload(history, src, ctx);
    }

    let mut encoder = ZstdEncoder::new();
    encoder
        .set_level(effective_oxiarc_level(ctx.compression_level))
        .set_content_size(ctx.fparams.contentSizeFlag != 0)
        .set_checksum(false);
    if !history.is_empty() {
        encoder.set_dictionary(history);
    }

    let encoded = encoder
        .compress(src)
        .map_err(|_| ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    let header = match parse_frame_header(&encoded, ZSTD_format_e::ZSTD_f_zstd1)? {
        HeaderProbe::Header(header) => header,
        HeaderProbe::Need(_) => return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC),
    };
    let start = header.headerSize as usize;
    if start > encoded.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }
    Ok(encoded[start..].to_vec())
}

fn write_stored_block(dst: &mut Vec<u8>, chunk: &[u8], last_block: bool) {
    if chunk.is_empty() {
        write_block_header(dst, last_block, 0, 0);
    } else if is_all_same(chunk) {
        write_block_header(dst, last_block, 1, chunk.len());
        dst.push(chunk[0]);
    } else {
        write_block_header(dst, last_block, 0, chunk.len());
        dst.extend_from_slice(chunk);
    }
}

fn append_stored_blocks(dst: &mut Vec<u8>, src: &[u8], block_size: usize, last_block: bool) {
    if src.is_empty() {
        if last_block {
            write_stored_block(dst, &[], true);
        }
        return;
    }

    let mut offset = 0usize;
    while offset < src.len() {
        let end = (offset + block_size).min(src.len());
        let is_last_chunk = end == src.len();
        write_stored_block(dst, &src[offset..end], last_block && is_last_chunk);
        offset = end;
    }
}

fn stream_content_size(ctx: &EncoderContext) -> Option<usize> {
    if ctx.fparams.contentSizeFlag == 0 {
        None
    } else if ctx.pledged_src_size != ZSTD_CONTENTSIZE_UNKNOWN {
        Some(ctx.pledged_src_size as usize)
    } else {
        None
    }
}

fn ensure_stream_header(ctx: &mut EncoderContext) -> Result<(), ZSTD_ErrorCode> {
    if ctx.stream.frame_started {
        return Ok(());
    }
    let header = build_frame_header(ctx, stream_content_size(ctx))?;
    append_pending(&mut ctx.stream, &header);
    ctx.stream.frame_started = true;
    Ok(())
}

fn finalize_deferred_stream_header(ctx: &mut EncoderContext) -> Result<(), ZSTD_ErrorCode> {
    if !ctx.stream.deferred_header {
        return Ok(());
    }
    let header = build_frame_header(ctx, Some(ctx.stream.input.len()))?;
    append_pending(&mut ctx.stream, &header[4..]);
    ctx.stream.deferred_header = false;
    Ok(())
}

fn queue_stream_chunk(ctx: &mut EncoderContext, src: &[u8]) -> Result<(), ZSTD_ErrorCode> {
    if src.is_empty() {
        return Ok(());
    }
    if ctx.stream.frame_finished {
        return Err(ZSTD_ErrorCode::ZSTD_error_init_missing);
    }

    let next_size = ctx.stream.input.len().saturating_add(src.len());
    if ctx.pledged_src_size != ZSTD_CONTENTSIZE_UNKNOWN && next_size > ctx.pledged_src_size as usize
    {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }
    ensure_stream_header(ctx)?;
    ctx.stream.input.extend_from_slice(src);
    Ok(())
}

fn pending_stream_segment<'a>(ctx: &'a EncoderContext) -> &'a [u8] {
    &ctx.stream.input[ctx.stream.emitted_input.min(ctx.stream.input.len())..]
}

fn append_stream_payload(
    ctx: &mut EncoderContext,
    src: &[u8],
    last_block: bool,
) -> Result<(), ZSTD_ErrorCode> {
    let block_size = ctx.frame_block_size();
    if src.is_empty() {
        if last_block {
            append_pending_stored_blocks(&mut ctx.stream, src, block_size, true);
        }
        return Ok(());
    }

    let prefer_stored_flush = !last_block && (ctx.nb_workers > 0 || !ctx.thread_pool.is_null());
    if prefer_stored_flush {
        append_pending_stored_blocks(&mut ctx.stream, src, block_size, false);
        return Ok(());
    }

    if ctx.stream.emitted_input == 0 {
        let history = compression_history(ctx)?
            .map(Cow::into_owned)
            .unwrap_or_default();
        let mut encoded = payload_with_history(&history, src, ctx)?;
        if !encoded.is_empty() && !last_block {
            let last_header_offset = last_block_header_offset(&encoded)?;
            encoded[last_header_offset] &= !1;
        }
        append_pending(&mut ctx.stream, &encoded);
    } else {
        append_pending_stored_blocks(&mut ctx.stream, src, block_size, last_block);
    }

    Ok(())
}

pub(crate) fn flush_stream_data(ctx: &mut EncoderContext) -> Result<(), ZSTD_ErrorCode> {
    if ctx.stream.frame_finished {
        return Err(ZSTD_ErrorCode::ZSTD_error_init_missing);
    }
    ensure_stream_header(ctx)?;
    finalize_deferred_stream_header(ctx)?;
    let segment = pending_stream_segment(ctx).to_vec();
    if segment.is_empty() {
        return Ok(());
    }
    append_stream_payload(ctx, &segment, false)?;
    ctx.stream.emitted_input = ctx.stream.input.len();
    Ok(())
}

fn copy_pending(stream: &mut StreamState, dst: &mut [u8], pos: &mut usize) -> usize {
    if *pos >= dst.len() || stream.pending_pos >= stream.pending.len() {
        return stream.pending.len().saturating_sub(stream.pending_pos);
    }

    let remaining = &stream.pending[stream.pending_pos..];
    let to_copy = remaining.len().min(dst.len() - *pos);
    dst[*pos..*pos + to_copy].copy_from_slice(&remaining[..to_copy]);
    *pos += to_copy;
    stream.pending_pos += to_copy;
    stream.flushed_total = stream.flushed_total.saturating_add(to_copy);

    if stream.pending_pos == stream.pending.len() {
        stream.pending.clear();
        stream.pending_pos = 0;
        return 0;
    }

    stream.pending.len().saturating_sub(stream.pending_pos)
}

pub(crate) fn stream_pending_bytes(ctx: &EncoderContext) -> usize {
    ctx.stream
        .pending
        .len()
        .saturating_sub(ctx.stream.pending_pos)
}

pub(crate) fn next_input_size_hint(ctx: &EncoderContext) -> usize {
    let block_size = ctx.frame_block_size().max(1);
    let buffered = ctx.stream.input.len() % block_size;
    if buffered == 0 {
        block_size
    } else {
        block_size - buffered
    }
}

pub(crate) fn flush_pending_to_dst(
    ctx: &mut EncoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
) -> Result<usize, ZSTD_ErrorCode> {
    let dst_slice = optional_src_slice_mut(dst, dst_capacity)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong)?;
    let remaining = ctx
        .stream
        .pending
        .len()
        .saturating_sub(ctx.stream.pending_pos);
    if remaining > dst_slice.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    let mut pos = 0usize;
    copy_pending(&mut ctx.stream, dst_slice, &mut pos);
    Ok(pos)
}

pub(crate) fn encode_frame(ctx: &EncoderContext, src: &[u8]) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    ctx.check_pledged_size(src.len())?;

    let mut frame = build_frame_header(ctx, Some(src.len()))?;
    if src.is_empty() {
        write_block_header(&mut frame, true, 0, 0);
    } else {
        let mut history = compression_history(ctx)?
            .map(Cow::into_owned)
            .unwrap_or_default();
        trim_history(&mut history, ctx.window_size().max(1));
        frame.extend_from_slice(&payload_with_history(&history, src, ctx)?);
    }

    if ctx.fparams.checksumFlag != 0 {
        frame.extend_from_slice(&(xxh64(src) as u32).to_le_bytes());
    }

    Ok(frame)
}

pub(crate) fn write_frame_to_dst(
    ctx: &EncoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    src: &[u8],
) -> Result<usize, ZSTD_ErrorCode> {
    let frame = encode_frame(ctx, src)?;
    let dst_slice = optional_src_slice_mut(dst, dst_capacity)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong)?;
    if frame.len() > dst_slice.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    dst_slice[..frame.len()].copy_from_slice(&frame);
    Ok(frame.len())
}

pub(crate) fn encode_block_body(
    ctx: &mut EncoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    src: &[u8],
) -> Result<usize, ZSTD_ErrorCode> {
    let dst_slice = optional_src_slice_mut(dst, dst_capacity)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong)?;
    if src.is_empty() {
        return Ok(0);
    }
    if is_all_same(src) {
        if dst_slice.is_empty() {
            return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
        }
        dst_slice[0] = src[0];
        ctx.push_block_history(src);
        return Ok(1);
    }

    let history = block_history_seed(ctx)?;
    let payload = payload_with_history(&history, src, ctx)?;
    let (block_type, body) = block_body_bounds(&payload)?;

    if block_type == 0 {
        ctx.push_block_history(src);
        return Ok(0);
    }
    if body.len() > dst_slice.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    dst_slice[..body.len()].copy_from_slice(body);
    ctx.push_block_history(src);
    Ok(body.len())
}

pub(crate) fn cparam_bounds(param: ZSTD_cParameter) -> ZSTD_bounds {
    match param {
        ZSTD_cParameter::ZSTD_c_compressionLevel => ZSTD_bounds {
            error: 0,
            lowerBound: min_clevel(),
            upperBound: ZSTD_MAX_CLEVEL,
        },
        ZSTD_cParameter::ZSTD_c_windowLog => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: 31,
        },
        ZSTD_cParameter::ZSTD_c_hashLog => ZSTD_bounds {
            error: 0,
            lowerBound: 6,
            upperBound: 30,
        },
        ZSTD_cParameter::ZSTD_c_chainLog => ZSTD_bounds {
            error: 0,
            lowerBound: 6,
            upperBound: 30,
        },
        ZSTD_cParameter::ZSTD_c_searchLog => ZSTD_bounds {
            error: 0,
            lowerBound: 1,
            upperBound: 30,
        },
        ZSTD_cParameter::ZSTD_c_minMatch => ZSTD_bounds {
            error: 0,
            lowerBound: 3,
            upperBound: 7,
        },
        ZSTD_cParameter::ZSTD_c_targetLength => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: ZSTD_BLOCKSIZE_MAX as c_int,
        },
        ZSTD_cParameter::ZSTD_c_strategy => ZSTD_bounds {
            error: 0,
            lowerBound: ZSTD_strategy::ZSTD_fast as c_int,
            upperBound: ZSTD_strategy::ZSTD_btultra2 as c_int,
        },
        ZSTD_cParameter::ZSTD_c_contentSizeFlag
        | ZSTD_cParameter::ZSTD_c_checksumFlag
        | ZSTD_cParameter::ZSTD_c_dictIDFlag => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: 1,
        },
        ZSTD_cParameter::ZSTD_c_enableLongDistanceMatching
        | ZSTD_cParameter::ZSTD_c_experimentalParam17 => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: 2,
        },
        ZSTD_cParameter::ZSTD_c_nbWorkers => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: 200,
        },
        ZSTD_cParameter::ZSTD_c_jobSize => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: i32::MAX,
        },
        ZSTD_cParameter::ZSTD_c_overlapLog => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: 9,
        },
        ZSTD_cParameter::ZSTD_c_experimentalParam11 => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: 1,
        },
        ZSTD_cParameter::ZSTD_c_experimentalParam12 => ZSTD_bounds {
            error: 0,
            lowerBound: 0,
            upperBound: 1,
        },
        _ => ZSTD_bounds {
            error: error_result(ZSTD_ErrorCode::ZSTD_error_parameter_unsupported),
            lowerBound: 0,
            upperBound: 0,
        },
    }
}

pub(crate) fn dparam_bounds(param: ZSTD_dParameter) -> ZSTD_bounds {
    match param {
        ZSTD_dParameter::ZSTD_d_windowLogMax => ZSTD_bounds {
            error: 0,
            lowerBound: 10,
            upperBound: 31,
        },
        _ => ZSTD_bounds {
            error: error_result(ZSTD_ErrorCode::ZSTD_error_parameter_unsupported),
            lowerBound: 0,
            upperBound: 0,
        },
    }
}

fn in_bounds(value: c_int, bounds: ZSTD_bounds) -> bool {
    !is_error_result(bounds.error) && value >= bounds.lowerBound && value <= bounds.upperBound
}

pub(crate) fn default_strategy_for_level(level: c_int) -> ZSTD_strategy {
    match level {
        i if i >= 15 => ZSTD_strategy::ZSTD_btultra2,
        i if i >= 12 => ZSTD_strategy::ZSTD_btultra,
        i if i >= 9 => ZSTD_strategy::ZSTD_btopt,
        i if i >= 6 => ZSTD_strategy::ZSTD_lazy2,
        i if i >= 4 => ZSTD_strategy::ZSTD_greedy,
        i if i >= 2 => ZSTD_strategy::ZSTD_dfast,
        _ => ZSTD_strategy::ZSTD_fast,
    }
}

pub(crate) fn normalize_cparams(
    mut params: ZSTD_compressionParameters,
) -> ZSTD_compressionParameters {
    let defaults = default_cparams();
    if !in_bounds(
        params.windowLog as c_int,
        cparam_bounds(ZSTD_cParameter::ZSTD_c_windowLog),
    ) {
        params.windowLog = defaults.windowLog;
    }
    if !in_bounds(
        params.hashLog as c_int,
        cparam_bounds(ZSTD_cParameter::ZSTD_c_hashLog),
    ) {
        params.hashLog = defaults.hashLog;
    }
    if !in_bounds(
        params.chainLog as c_int,
        cparam_bounds(ZSTD_cParameter::ZSTD_c_chainLog),
    ) {
        params.chainLog = defaults.chainLog;
    }
    if !in_bounds(
        params.searchLog as c_int,
        cparam_bounds(ZSTD_cParameter::ZSTD_c_searchLog),
    ) {
        params.searchLog = defaults.searchLog;
    }
    if !in_bounds(
        params.minMatch as c_int,
        cparam_bounds(ZSTD_cParameter::ZSTD_c_minMatch),
    ) {
        params.minMatch = defaults.minMatch;
    }
    if !in_bounds(
        params.targetLength as c_int,
        cparam_bounds(ZSTD_cParameter::ZSTD_c_targetLength),
    ) {
        params.targetLength = defaults.targetLength;
    }
    if !in_bounds(
        params.strategy as c_int,
        cparam_bounds(ZSTD_cParameter::ZSTD_c_strategy),
    ) {
        params.strategy = defaults.strategy;
    }
    params
}

pub(crate) fn get_cparams(
    compression_level: c_int,
    estimated_src_size: u64,
    dict_size: usize,
) -> ZSTD_compressionParameters {
    let estimated_src_size = normalize_src_size_hint(estimated_src_size);
    let effective_size = if estimated_src_size == ZSTD_CONTENTSIZE_UNKNOWN {
        if dict_size == 0 {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            dict_size as u64 + 500
        }
    } else {
        estimated_src_size.saturating_add(dict_size as u64)
    };
    let level = normalize_compression_level(compression_level);
    if level < 0 {
        return normalize_cparams(negative_level_cparams(effective_size, (-level) as u32));
    }

    let mut params = default_cparams();
    let approx_window = if effective_size == ZSTD_CONTENTSIZE_UNKNOWN {
        19
    } else {
        let needed = 64 - effective_size.max(1).leading_zeros();
        needed.clamp(10, 27)
    };
    params.windowLog = approx_window;
    params.hashLog = approx_window.saturating_sub(1);
    params.chainLog = approx_window.saturating_sub(1);
    params.searchLog = if level >= 15 {
        7
    } else if level >= 12 {
        6
    } else if level >= 9 {
        5
    } else if level >= 6 {
        4
    } else if level >= 4 {
        2
    } else {
        1
    };
    params.minMatch = if level >= 15 {
        6
    } else if level >= 6 {
        5
    } else {
        4
    };
    params.targetLength = if level >= 15 {
        64
    } else if level >= 9 {
        16
    } else if level >= 6 {
        8
    } else {
        0
    };
    params.strategy = default_strategy_for_level(level);
    normalize_cparams(params)
}

pub(crate) fn get_params(
    compression_level: c_int,
    estimated_src_size: u64,
    dict_size: usize,
) -> ZSTD_parameters {
    ZSTD_parameters {
        cParams: get_cparams(compression_level, estimated_src_size, dict_size),
        fParams: default_params().fParams,
    }
}

pub(crate) fn check_cparams(params: ZSTD_compressionParameters) -> Result<(), ZSTD_ErrorCode> {
    let checks = [
        (params.windowLog as c_int, ZSTD_cParameter::ZSTD_c_windowLog),
        (params.hashLog as c_int, ZSTD_cParameter::ZSTD_c_hashLog),
        (params.chainLog as c_int, ZSTD_cParameter::ZSTD_c_chainLog),
        (params.searchLog as c_int, ZSTD_cParameter::ZSTD_c_searchLog),
        (params.minMatch as c_int, ZSTD_cParameter::ZSTD_c_minMatch),
        (
            params.targetLength as c_int,
            ZSTD_cParameter::ZSTD_c_targetLength,
        ),
        (params.strategy as c_int, ZSTD_cParameter::ZSTD_c_strategy),
    ];
    for (value, param) in checks {
        if !in_bounds(value, cparam_bounds(param)) {
            return Err(ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound);
        }
    }
    Ok(())
}

pub(crate) fn adjust_cparams(
    mut cparams: ZSTD_compressionParameters,
    src_size: u64,
    dict_size: usize,
) -> ZSTD_compressionParameters {
    const HASHLOG_MIN: u32 = 6;
    const MAX_WINDOW_RESIZE: u64 = 1 << 30;

    let bounds = |param| cparam_bounds(param);
    cparams.windowLog = (cparams.windowLog as c_int).clamp(
        bounds(ZSTD_cParameter::ZSTD_c_windowLog).lowerBound,
        bounds(ZSTD_cParameter::ZSTD_c_windowLog).upperBound,
    ) as u32;
    cparams.hashLog = (cparams.hashLog as c_int).clamp(
        bounds(ZSTD_cParameter::ZSTD_c_hashLog).lowerBound,
        bounds(ZSTD_cParameter::ZSTD_c_hashLog).upperBound,
    ) as u32;
    cparams.chainLog = (cparams.chainLog as c_int).clamp(
        bounds(ZSTD_cParameter::ZSTD_c_chainLog).lowerBound,
        bounds(ZSTD_cParameter::ZSTD_c_chainLog).upperBound,
    ) as u32;
    cparams.searchLog = (cparams.searchLog as c_int).clamp(
        bounds(ZSTD_cParameter::ZSTD_c_searchLog).lowerBound,
        bounds(ZSTD_cParameter::ZSTD_c_searchLog).upperBound,
    ) as u32;
    cparams.minMatch = (cparams.minMatch as c_int).clamp(
        bounds(ZSTD_cParameter::ZSTD_c_minMatch).lowerBound,
        bounds(ZSTD_cParameter::ZSTD_c_minMatch).upperBound,
    ) as u32;
    cparams.targetLength = (cparams.targetLength as c_int).clamp(
        bounds(ZSTD_cParameter::ZSTD_c_targetLength).lowerBound,
        bounds(ZSTD_cParameter::ZSTD_c_targetLength).upperBound,
    ) as u32;
    cparams.strategy = match (cparams.strategy as c_int).clamp(
        bounds(ZSTD_cParameter::ZSTD_c_strategy).lowerBound,
        bounds(ZSTD_cParameter::ZSTD_c_strategy).upperBound,
    ) {
        1 => ZSTD_strategy::ZSTD_fast,
        2 => ZSTD_strategy::ZSTD_dfast,
        3 => ZSTD_strategy::ZSTD_greedy,
        4 => ZSTD_strategy::ZSTD_lazy,
        5 => ZSTD_strategy::ZSTD_lazy2,
        6 => ZSTD_strategy::ZSTD_btlazy2,
        7 => ZSTD_strategy::ZSTD_btopt,
        8 => ZSTD_strategy::ZSTD_btultra,
        _ => ZSTD_strategy::ZSTD_btultra2,
    };

    let normalized_src_size = normalize_src_size_hint(src_size);
    if normalized_src_size <= MAX_WINDOW_RESIZE && (dict_size as u64) <= MAX_WINDOW_RESIZE {
        let total_size = normalized_src_size.saturating_add(dict_size as u64);
        let src_log = if total_size < (1u64 << HASHLOG_MIN) {
            HASHLOG_MIN
        } else {
            u64::BITS - (total_size - 1).leading_zeros()
        };
        cparams.windowLog = cparams.windowLog.min(src_log);
    }

    if normalized_src_size != ZSTD_CONTENTSIZE_UNKNOWN {
        let covered = normalized_src_size.saturating_add(dict_size as u64).max(1);
        let covered_log = (u64::BITS - (covered - 1).leading_zeros())
            .max(cparam_bounds(ZSTD_cParameter::ZSTD_c_windowLog).lowerBound as u32);
        cparams.hashLog = cparams.hashLog.min(covered_log.saturating_add(1));
        cparams.chainLog = cparams.chainLog.min(covered_log);
    }

    normalize_cparams(cparams)
}

pub(crate) fn set_parameter(
    ctx: &mut EncoderContext,
    param: ZSTD_cParameter,
    value: c_int,
) -> Result<(), ZSTD_ErrorCode> {
    let bounds = cparam_bounds(param);
    if is_error_result(bounds.error) {
        return Err(ZSTD_ErrorCode::ZSTD_error_parameter_unsupported);
    }
    if value < bounds.lowerBound || value > bounds.upperBound {
        return Err(ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound);
    }

    match param {
        ZSTD_cParameter::ZSTD_c_compressionLevel => {
            let level = normalize_compression_level(value);
            let dict_size = ctx.dict.as_ref().map_or(0, EncoderDictionary::len);
            ctx.compression_level = level;
            ctx.cparams = get_cparams(level, ctx.pledged_src_size, dict_size);
        }
        ZSTD_cParameter::ZSTD_c_windowLog => {
            if value == 0 {
                let dict_size = ctx.dict.as_ref().map_or(0, EncoderDictionary::len);
                ctx.cparams.windowLog =
                    get_cparams(ctx.compression_level, ctx.pledged_src_size, dict_size).windowLog;
            } else {
                ctx.cparams.windowLog = value as u32;
            }
        }
        ZSTD_cParameter::ZSTD_c_hashLog => ctx.cparams.hashLog = value as u32,
        ZSTD_cParameter::ZSTD_c_chainLog => ctx.cparams.chainLog = value as u32,
        ZSTD_cParameter::ZSTD_c_searchLog => ctx.cparams.searchLog = value as u32,
        ZSTD_cParameter::ZSTD_c_minMatch => ctx.cparams.minMatch = value as u32,
        ZSTD_cParameter::ZSTD_c_targetLength => ctx.cparams.targetLength = value as u32,
        ZSTD_cParameter::ZSTD_c_strategy => {
            ctx.cparams.strategy = match value {
                1 => ZSTD_strategy::ZSTD_fast,
                2 => ZSTD_strategy::ZSTD_dfast,
                3 => ZSTD_strategy::ZSTD_greedy,
                4 => ZSTD_strategy::ZSTD_lazy,
                5 => ZSTD_strategy::ZSTD_lazy2,
                6 => ZSTD_strategy::ZSTD_btlazy2,
                7 => ZSTD_strategy::ZSTD_btopt,
                8 => ZSTD_strategy::ZSTD_btultra,
                _ => ZSTD_strategy::ZSTD_btultra2,
            };
        }
        ZSTD_cParameter::ZSTD_c_enableLongDistanceMatching => {
            ctx.enable_long_distance_matching = value == 1
        }
        ZSTD_cParameter::ZSTD_c_contentSizeFlag => {
            ctx.fparams.contentSizeFlag = i32::from(value != 0)
        }
        ZSTD_cParameter::ZSTD_c_checksumFlag => ctx.fparams.checksumFlag = i32::from(value != 0),
        ZSTD_cParameter::ZSTD_c_dictIDFlag => ctx.fparams.noDictIDFlag = i32::from(value == 0),
        ZSTD_cParameter::ZSTD_c_nbWorkers => ctx.nb_workers = value,
        ZSTD_cParameter::ZSTD_c_jobSize => ctx.job_size = value,
        ZSTD_cParameter::ZSTD_c_overlapLog => ctx.overlap_log = value,
        ZSTD_cParameter::ZSTD_c_experimentalParam11 => {
            ctx.block_delimiters =
                if value == ZSTD_sequenceFormat_e::ZSTD_sf_explicitBlockDelimiters as c_int {
                    ZSTD_sequenceFormat_e::ZSTD_sf_explicitBlockDelimiters
                } else {
                    ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters
                };
        }
        ZSTD_cParameter::ZSTD_c_experimentalParam12 => ctx.validate_sequences = value != 0,
        ZSTD_cParameter::ZSTD_c_experimentalParam17 => {
            ctx.enable_seq_producer_fallback = value == 1;
        }
        _ => return Err(ZSTD_ErrorCode::ZSTD_error_parameter_unsupported),
    }

    Ok(())
}

pub(crate) fn get_parameter(
    ctx: &EncoderContext,
    param: ZSTD_cParameter,
) -> Result<c_int, ZSTD_ErrorCode> {
    Ok(match param {
        ZSTD_cParameter::ZSTD_c_compressionLevel => ctx.compression_level,
        ZSTD_cParameter::ZSTD_c_windowLog => ctx.cparams.windowLog as c_int,
        ZSTD_cParameter::ZSTD_c_hashLog => ctx.cparams.hashLog as c_int,
        ZSTD_cParameter::ZSTD_c_chainLog => ctx.cparams.chainLog as c_int,
        ZSTD_cParameter::ZSTD_c_searchLog => ctx.cparams.searchLog as c_int,
        ZSTD_cParameter::ZSTD_c_minMatch => ctx.cparams.minMatch as c_int,
        ZSTD_cParameter::ZSTD_c_targetLength => ctx.cparams.targetLength as c_int,
        ZSTD_cParameter::ZSTD_c_strategy => ctx.cparams.strategy as c_int,
        ZSTD_cParameter::ZSTD_c_enableLongDistanceMatching => {
            i32::from(ctx.enable_long_distance_matching)
        }
        ZSTD_cParameter::ZSTD_c_contentSizeFlag => ctx.fparams.contentSizeFlag,
        ZSTD_cParameter::ZSTD_c_checksumFlag => ctx.fparams.checksumFlag,
        ZSTD_cParameter::ZSTD_c_dictIDFlag => i32::from(ctx.fparams.noDictIDFlag == 0),
        ZSTD_cParameter::ZSTD_c_nbWorkers => ctx.nb_workers,
        ZSTD_cParameter::ZSTD_c_jobSize => ctx.job_size,
        ZSTD_cParameter::ZSTD_c_overlapLog => ctx.overlap_log,
        ZSTD_cParameter::ZSTD_c_experimentalParam11 => ctx.block_delimiters as c_int,
        ZSTD_cParameter::ZSTD_c_experimentalParam12 => i32::from(ctx.validate_sequences),
        ZSTD_cParameter::ZSTD_c_experimentalParam17 => i32::from(ctx.enable_seq_producer_fallback),
        _ => return Err(ZSTD_ErrorCode::ZSTD_error_parameter_unsupported),
    })
}

pub(crate) fn set_sequence_producer(
    ctx: &mut EncoderContext,
    sequence_producer_state: *mut c_void,
    sequence_producer: Option<ZSTD_sequenceProducer_F>,
) {
    ctx.sequence_producer_state = sequence_producer_state;
    ctx.sequence_producer = sequence_producer;
}

pub(crate) fn sequence_bound(src_size: usize) -> usize {
    src_size / 3 + 1
}

#[derive(Clone, Debug, Default)]
struct ExplicitSequenceBlock {
    size: usize,
    sequences: Vec<ZSTD_Sequence>,
}

struct ExternalSequenceMatcher {
    blocks: Vec<ExplicitSequenceBlock>,
    block_index: usize,
    last_space: Vec<u8>,
    window_size: usize,
}

impl ExternalSequenceMatcher {
    fn new(blocks: Vec<ExplicitSequenceBlock>, window_size: usize) -> Self {
        Self {
            blocks,
            block_index: 0,
            last_space: Vec::new(),
            window_size: window_size.max(1),
        }
    }
}

impl Matcher for ExternalSequenceMatcher {
    fn get_next_space(&mut self) -> Vec<u8> {
        let size = self
            .blocks
            .get(self.block_index)
            .map_or(1usize, |block| block.size.max(1));
        vec![0u8; size]
    }

    fn get_last_space(&mut self) -> &[u8] {
        self.last_space.as_slice()
    }

    fn commit_space(&mut self, space: Vec<u8>) {
        self.last_space = space;
    }

    fn skip_matching(&mut self) {
        self.block_index = self.block_index.saturating_add(1);
    }

    fn start_matching(&mut self, mut handle_sequence: impl for<'a> FnMut(StructuredSequence<'a>)) {
        let Some(block) = self.blocks.get(self.block_index) else {
            return;
        };
        let data = self.last_space.as_slice();
        let mut cursor = 0usize;

        for sequence in &block.sequences {
            let literal_length = sequence.litLength as usize;
            let match_length = sequence.matchLength as usize;
            if sequence.offset == 0 && match_length == 0 {
                let literal_end = cursor.saturating_add(literal_length).min(data.len());
                handle_sequence(StructuredSequence::Literals {
                    literals: &data[cursor..literal_end],
                });
                cursor = literal_end;
                continue;
            }

            let literal_end = cursor.saturating_add(literal_length).min(data.len());
            handle_sequence(StructuredSequence::Triple {
                literals: &data[cursor..literal_end],
                offset: sequence.offset as usize,
                match_len: match_length,
            });
            cursor = cursor.saturating_add(literal_length.saturating_add(match_length));
        }

        if cursor < data.len() {
            handle_sequence(StructuredSequence::Literals {
                literals: &data[cursor..],
            });
        }
        self.block_index = self.block_index.saturating_add(1);
    }

    fn reset(&mut self, _level: StructuredCompressionLevel) {
        self.block_index = 0;
        self.last_space.clear();
    }

    fn window_size(&self) -> u64 {
        self.window_size as u64
    }
}

#[derive(Default)]
struct SequenceFramePlan {
    blocks: Vec<ExplicitSequenceBlock>,
}

fn sequence_slice<'a>(
    sequences: *const ZSTD_Sequence,
    sequence_count: usize,
) -> Result<&'a [ZSTD_Sequence], ZSTD_ErrorCode> {
    if sequence_count == 0 {
        return Ok(&[]);
    }
    if sequences.is_null() {
        return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
    }
    Ok(unsafe { core::slice::from_raw_parts(sequences, sequence_count) })
}

fn push_generated_sequence(
    out_sequences: *mut ZSTD_Sequence,
    out_capacity: usize,
    count: &mut usize,
    sequence: ZSTD_Sequence,
) -> Result<(), ZSTD_ErrorCode> {
    if out_sequences.is_null() || *count >= out_capacity {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    unsafe {
        out_sequences.add(*count).write(sequence);
    }
    *count += 1;
    Ok(())
}

fn zero_sequence(lit_length: usize) -> Result<ZSTD_Sequence, ZSTD_ErrorCode> {
    Ok(ZSTD_Sequence {
        offset: 0,
        litLength: lit_length
            .try_into()
            .map_err(|_| ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?,
        matchLength: 0,
        rep: 0,
    })
}

fn postprocess_external_sequences(
    out_sequences: *mut ZSTD_Sequence,
    out_capacity: usize,
    produced: usize,
    src_size: usize,
) -> Result<usize, ZSTD_ErrorCode> {
    if produced > out_capacity {
        return Err(ZSTD_ErrorCode::ZSTD_error_sequenceProducer_failed);
    }
    if produced == 0 && src_size > 0 {
        return Err(ZSTD_ErrorCode::ZSTD_error_sequenceProducer_failed);
    }
    if src_size == 0 {
        if out_capacity == 0 {
            return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
        }
        unsafe {
            out_sequences.write(zero_sequence(0)?);
        }
        return Ok(1);
    }
    if produced == 0 {
        return Err(ZSTD_ErrorCode::ZSTD_error_sequenceProducer_failed);
    }
    if out_sequences.is_null() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    let last = unsafe { *out_sequences.add(produced - 1) };
    if last.offset == 0 && last.matchLength == 0 {
        return Ok(produced);
    }
    if produced == out_capacity {
        return Err(ZSTD_ErrorCode::ZSTD_error_sequenceProducer_failed);
    }
    unsafe {
        out_sequences.add(produced).write(zero_sequence(0)?);
    }
    Ok(produced + 1)
}

fn sequence_history_seed(ctx: &EncoderContext) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let mut history = compression_history(ctx)?
        .map(Cow::into_owned)
        .unwrap_or_default();
    trim_history(&mut history, ctx.window_size().max(1));
    Ok(history)
}

fn commit_matcher_bytes<M: Matcher>(matcher: &mut M, bytes: &[u8]) {
    let mut space = matcher.get_next_space();
    if space.len() < bytes.len() {
        space.resize(bytes.len(), 0);
    } else {
        space.truncate(bytes.len());
    }
    space[..bytes.len()].copy_from_slice(bytes);
    matcher.commit_space(space);
}

fn generate_sequences_internal(
    ctx: &EncoderContext,
    out_sequences: *mut ZSTD_Sequence,
    out_capacity: usize,
    src: &[u8],
) -> Result<usize, ZSTD_ErrorCode> {
    if src.is_empty() {
        let mut count = 0usize;
        push_generated_sequence(out_sequences, out_capacity, &mut count, zero_sequence(0)?)?;
        return Ok(count);
    }

    let mut history = sequence_history_seed(ctx)?;
    let block_size = ctx.frame_block_size().max(1);
    let mut matcher = DictionaryMatcher::new(
        history.clone(),
        block_size,
        ctx.window_size().max(block_size),
        normalize_compression_level(ctx.compression_level).max(1),
    );
    matcher.reset(structured_level(ctx.compression_level));
    let mut count = 0usize;

    for block in src.chunks(block_size) {
        commit_matcher_bytes(&mut matcher, block);
        let mut result = Ok(());
        let mut block_ended_with_literals = false;

        matcher.start_matching(|sequence| {
            if result.is_err() {
                return;
            }
            match sequence {
                StructuredSequence::Triple {
                    literals,
                    offset,
                    match_len,
                } => {
                    block_ended_with_literals = false;
                    result = push_generated_sequence(
                        out_sequences,
                        out_capacity,
                        &mut count,
                        ZSTD_Sequence {
                            offset: match offset.try_into() {
                                Ok(value) => value,
                                Err(_) => {
                                    result =
                                        Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
                                    return;
                                }
                            },
                            litLength: match literals.len().try_into() {
                                Ok(value) => value,
                                Err(_) => {
                                    result =
                                        Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
                                    return;
                                }
                            },
                            matchLength: match match_len.try_into() {
                                Ok(value) => value,
                                Err(_) => {
                                    result =
                                        Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
                                    return;
                                }
                            },
                            rep: 0,
                        },
                    );
                }
                StructuredSequence::Literals { literals } => {
                    block_ended_with_literals = true;
                    result = zero_sequence(literals.len()).and_then(|sequence| {
                        push_generated_sequence(out_sequences, out_capacity, &mut count, sequence)
                    });
                }
            }
        });
        result?;

        if !block_ended_with_literals {
            push_generated_sequence(out_sequences, out_capacity, &mut count, zero_sequence(0)?)?;
        }

        history.extend_from_slice(block);
        trim_history(&mut history, ctx.window_size().max(1));
    }

    Ok(count)
}

fn validate_external_sequence(
    ctx: &EncoderContext,
    history: &[u8],
    src: &[u8],
    produced: usize,
    literal_length: usize,
    match_length: usize,
    offset: usize,
) -> Result<(), ZSTD_ErrorCode> {
    if match_length < ctx.cparams.minMatch as usize || offset == 0 {
        return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
    }
    let match_start = produced
        .checked_add(literal_length)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?;
    let match_end = match_start
        .checked_add(match_length)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?;
    if match_end > src.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
    }

    let available_history = history.len().saturating_add(match_start);
    if offset > available_history {
        return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
    }

    for idx in 0..match_length {
        let dst_index = match_start + idx;
        let ref_index = history.len() + dst_index - offset;
        let expected = if ref_index < history.len() {
            history[ref_index]
        } else {
            src[ref_index - history.len()]
        };
        if src[dst_index] != expected {
            return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
        }
    }

    Ok(())
}

fn plan_sequence_frame(
    ctx: &EncoderContext,
    sequences: &[ZSTD_Sequence],
    src: &[u8],
) -> Result<SequenceFramePlan, ZSTD_ErrorCode> {
    let mut plan = SequenceFramePlan::default();
    let mut produced = 0usize;
    let mut current_block = 0usize;
    let mut block_sequences = Vec::new();
    let explicit_delimiters =
        ctx.block_delimiters == ZSTD_sequenceFormat_e::ZSTD_sf_explicitBlockDelimiters;
    let history = sequence_history_seed(ctx)?;
    let max_block_size = ctx.frame_block_size().max(1);
    let flush_block = |plan: &mut SequenceFramePlan,
                       current_block: &mut usize,
                       block_sequences: &mut Vec<ZSTD_Sequence>| {
        if *current_block == 0 && block_sequences.is_empty() {
            return;
        }
        plan.blocks.push(ExplicitSequenceBlock {
            size: *current_block,
            sequences: core::mem::take(block_sequences),
        });
        *current_block = 0;
    };

    for sequence in sequences {
        let literal_length = sequence.litLength as usize;
        let match_length = sequence.matchLength as usize;

        if sequence.offset == 0 && match_length == 0 {
            produced = produced
                .checked_add(literal_length)
                .ok_or(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?;
            if produced > src.len() {
                return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
            }

            if explicit_delimiters {
                current_block = current_block
                    .checked_add(literal_length)
                    .ok_or(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?;
                if current_block > max_block_size {
                    return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
                }
                if current_block != 0 || produced == src.len() {
                    block_sequences.push(*sequence);
                    flush_block(&mut plan, &mut current_block, &mut block_sequences);
                }
            } else {
                let mut remaining = literal_length;
                while remaining != 0 {
                    if current_block == max_block_size {
                        flush_block(&mut plan, &mut current_block, &mut block_sequences);
                    }
                    let take = remaining.min(max_block_size - current_block);
                    block_sequences.push(zero_sequence(take)?);
                    current_block += take;
                    remaining -= take;
                    if current_block == max_block_size {
                        flush_block(&mut plan, &mut current_block, &mut block_sequences);
                    }
                }
            }
            continue;
        }

        let sequence_bytes = literal_length
            .checked_add(match_length)
            .ok_or(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?;
        let end = produced
            .checked_add(sequence_bytes)
            .ok_or(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?;
        if end > src.len() {
            return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
        }
        if ctx.validate_sequences {
            validate_external_sequence(
                ctx,
                history.as_slice(),
                src,
                produced,
                literal_length,
                match_length,
                sequence.offset as usize,
            )?;
        }
        produced = end;
        if sequence_bytes > max_block_size {
            return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
        }
        if current_block != 0 && current_block + sequence_bytes > max_block_size {
            if explicit_delimiters {
                return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
            }
            flush_block(&mut plan, &mut current_block, &mut block_sequences);
        }
        current_block = current_block
            .checked_add(sequence_bytes)
            .ok_or(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid)?;
        if explicit_delimiters && current_block > max_block_size {
            return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
        }
        block_sequences.push(*sequence);
        if !explicit_delimiters && current_block == max_block_size {
            flush_block(&mut plan, &mut current_block, &mut block_sequences);
        }
    }

    if current_block != 0 {
        if current_block > max_block_size {
            return Err(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
        }
        flush_block(&mut plan, &mut current_block, &mut block_sequences);
    }

    let mut remaining = src.len().saturating_sub(produced);
    while remaining != 0 {
        let chunk_size = min(remaining, max_block_size);
        plan.blocks.push(ExplicitSequenceBlock {
            size: chunk_size,
            sequences: Vec::new(),
        });
        remaining -= chunk_size;
    }

    Ok(plan)
}

fn explicit_sequence_payload(
    ctx: &EncoderContext,
    blocks: &[ExplicitSequenceBlock],
    src: &[u8],
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let matcher = ExternalSequenceMatcher::new(blocks.to_vec(), ctx.window_size().max(1));
    let mut compressor =
        FrameCompressor::new_with_matcher(matcher, structured_level(ctx.compression_level));
    let mut encoded = Vec::with_capacity(src.len().saturating_add(32));
    compressor.set_source(src);
    compressor.set_drain(&mut encoded);
    compressor.compress();

    let header = match parse_frame_header(&encoded, ZSTD_format_e::ZSTD_f_zstd1)? {
        HeaderProbe::Header(header) => header,
        HeaderProbe::Need(_) => return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC),
    };
    let start = header.headerSize as usize;
    let trailer = usize::from(header.checksumFlag != 0) * 4;
    if start > encoded.len() || start + trailer > encoded.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }
    Ok(encoded[start..encoded.len() - trailer].to_vec())
}

pub(crate) fn compress_sequences_to_dst(
    ctx: &EncoderContext,
    dst: *mut c_void,
    dst_capacity: usize,
    in_sequences: *const ZSTD_Sequence,
    in_sequence_count: usize,
    src: &[u8],
) -> Result<usize, ZSTD_ErrorCode> {
    let sequences = sequence_slice(in_sequences, in_sequence_count)?;
    let plan = plan_sequence_frame(ctx, sequences, src)?;
    let mut frame = build_frame_header(ctx, Some(src.len()))?;

    if src.is_empty() {
        write_block_header(&mut frame, true, 0, 0);
    } else {
        frame.extend_from_slice(&explicit_sequence_payload(ctx, &plan.blocks, src)?);
    }

    if ctx.fparams.checksumFlag != 0 {
        frame.extend_from_slice(&(xxh64(src) as u32).to_le_bytes());
    }

    let dst_slice = optional_src_slice_mut(dst, dst_capacity)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong)?;
    if frame.len() > dst_slice.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    dst_slice[..frame.len()].copy_from_slice(&frame);
    Ok(frame.len())
}

pub(crate) fn emit_sequences(
    ctx: &EncoderContext,
    out_sequences: *mut ZSTD_Sequence,
    out_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> Result<usize, ZSTD_ErrorCode> {
    let src =
        optional_src_slice(src, src_size).ok_or(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong)?;
    if let Some(producer) = ctx.sequence_producer {
        let produced = unsafe {
            producer(
                ctx.sequence_producer_state,
                out_sequences,
                out_capacity,
                src.as_ptr().cast(),
                src.len(),
                ctx.dict
                    .as_ref()
                    .map_or(core::ptr::null(), |dict| dict.bytes().as_ptr().cast()),
                ctx.dict.as_ref().map_or(0, EncoderDictionary::len),
                ctx.compression_level,
                1usize << ctx.cparams.windowLog.min(30),
            )
        };
        match postprocess_external_sequences(out_sequences, out_capacity, produced, src.len()) {
            Ok(produced) => return Ok(produced),
            Err(error) if !ctx.enable_seq_producer_fallback => return Err(error),
            Err(_) => {}
        }
    }
    generate_sequences_internal(ctx, out_sequences, out_capacity, src)
}

pub(crate) fn legacy_begin(ctx: &mut EncoderContext) {
    ctx.clear_session();
    ctx.legacy_mode = true;
}

pub(crate) fn stage_legacy_input(
    ctx: &mut EncoderContext,
    src: &[u8],
    last_block: bool,
) -> Result<(), ZSTD_ErrorCode> {
    let block_size = ctx.frame_block_size();
    if ctx.stream.frame_finished {
        return Err(ZSTD_ErrorCode::ZSTD_error_init_missing);
    }

    let next_size = ctx.stream.input.len().saturating_add(src.len());
    if ctx.pledged_src_size != ZSTD_CONTENTSIZE_UNKNOWN && next_size > ctx.pledged_src_size as usize
    {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }
    if last_block
        && ctx.pledged_src_size != ZSTD_CONTENTSIZE_UNKNOWN
        && next_size != ctx.pledged_src_size as usize
    {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    ensure_stream_header(ctx)?;
    ctx.stream.input.extend_from_slice(src);
    if src.is_empty() {
        if last_block {
            append_pending_stored_blocks(&mut ctx.stream, src, block_size, true);
        }
    } else {
        append_pending_stored_blocks(&mut ctx.stream, src, block_size, last_block);
        ctx.stream.emitted_input = ctx.stream.input.len();
    }
    if last_block {
        if ctx.fparams.checksumFlag != 0 {
            let checksum = (xxh64(&ctx.stream.input) as u32).to_le_bytes();
            append_pending(&mut ctx.stream, &checksum);
        }
        ctx.stream.frame_finished = true;
    }
    Ok(())
}

pub(crate) fn stage_src_slice(ctx: &mut EncoderContext, src: &[u8]) -> Result<(), ZSTD_ErrorCode> {
    queue_stream_chunk(ctx, src)
}

pub(crate) fn stage_stream_input(
    ctx: &mut EncoderContext,
    input: *mut ZSTD_inBuffer,
) -> Result<(), ZSTD_ErrorCode> {
    let input = unsafe { input.as_mut() }.ok_or(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong)?;
    let src = optional_src_slice(input.src, input.size)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong)?;
    let remaining = &src[input.pos.min(src.len())..];
    queue_stream_chunk(ctx, remaining)?;
    input.pos = input.size;
    Ok(())
}

pub(crate) fn finalize_stream(ctx: &mut EncoderContext) -> Result<(), ZSTD_ErrorCode> {
    if ctx.stream.frame_finished {
        return Ok(());
    }

    if ctx.pledged_src_size != ZSTD_CONTENTSIZE_UNKNOWN
        && ctx.stream.input.len() != ctx.pledged_src_size as usize
    {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    ensure_stream_header(ctx)?;
    finalize_deferred_stream_header(ctx)?;
    let segment = pending_stream_segment(ctx).to_vec();
    if segment.is_empty() {
        let mut trailer = Vec::with_capacity(BLOCK_HEADER_SIZE + 4);
        write_block_header(&mut trailer, true, 0, 0);
        if ctx.fparams.checksumFlag != 0 {
            trailer.extend_from_slice(&(xxh64(&ctx.stream.input) as u32).to_le_bytes());
        }
        append_pending(&mut ctx.stream, &trailer);
    } else {
        append_stream_payload(ctx, &segment, true)?;
        ctx.stream.emitted_input = ctx.stream.input.len();
        if ctx.fparams.checksumFlag != 0 {
            let checksum = (xxh64(&ctx.stream.input) as u32).to_le_bytes();
            append_pending(&mut ctx.stream, &checksum);
        }
    }
    ctx.stream.frame_finished = true;
    Ok(())
}

pub(crate) fn flush_stream_output(
    ctx: &mut EncoderContext,
    output: *mut ZSTD_outBuffer,
) -> Result<usize, ZSTD_ErrorCode> {
    let output = unsafe { output.as_mut() }.ok_or(ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong)?;
    let dst = optional_src_slice_mut(output.dst, output.size)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong)?;
    Ok(copy_pending(&mut ctx.stream, dst, &mut output.pos))
}

fn estimate_window_size(cparams: ZSTD_compressionParameters) -> usize {
    1usize << cparams.windowLog.min(30)
}

fn estimate_block_size(cparams: ZSTD_compressionParameters) -> usize {
    estimate_window_size(cparams).min(ZSTD_BLOCKSIZE_MAX)
}

fn estimate_sequence_count(block_size: usize, min_match: u32) -> usize {
    let stride = usize::max(min_match as usize, 3);
    block_size.div_ceil(stride).saturating_add(8)
}

fn estimate_match_state_bytes(cparams: ZSTD_compressionParameters) -> usize {
    let hash_bytes =
        (1usize << cparams.hashLog.min(30)).saturating_mul(core::mem::size_of::<u32>());
    let chain_bytes = match cparams.strategy {
        ZSTD_strategy::ZSTD_fast => 0,
        ZSTD_strategy::ZSTD_dfast | ZSTD_strategy::ZSTD_greedy => {
            (1usize << cparams.chainLog.min(30)).saturating_mul(core::mem::size_of::<u32>())
        }
        ZSTD_strategy::ZSTD_lazy
        | ZSTD_strategy::ZSTD_lazy2
        | ZSTD_strategy::ZSTD_btlazy2
        | ZSTD_strategy::ZSTD_btopt
        | ZSTD_strategy::ZSTD_btultra
        | ZSTD_strategy::ZSTD_btultra2 => {
            let chain_entries = 1usize << cparams.chainLog.min(30);
            chain_entries
                .saturating_mul(core::mem::size_of::<u32>())
                .saturating_add(hash_bytes / 2)
        }
    };
    hash_bytes.saturating_add(chain_bytes)
}

fn estimate_ldm_bytes(ctx: &EncoderContext, block_size: usize) -> usize {
    if !ctx.enable_long_distance_matching {
        return 0;
    }
    let ldm_hash_log = ctx
        .cparams
        .hashLog
        .max(ctx.cparams.windowLog.saturating_sub(1))
        .clamp(6, 30);
    let table_bytes = (1usize << ldm_hash_log).saturating_mul(core::mem::size_of::<u32>());
    table_bytes.saturating_add(block_size / 2)
}

fn estimate_token_bytes(cparams: ZSTD_compressionParameters) -> usize {
    let block_size = estimate_block_size(cparams);
    let seq_count = estimate_sequence_count(block_size, cparams.minMatch);
    block_size.saturating_add(seq_count.saturating_mul(core::mem::size_of::<u32>() * 4))
}

fn estimate_cctx_size_from_context(ctx: &EncoderContext) -> usize {
    let block_size = estimate_block_size(ctx.cparams);
    alloc::base_size::<EncoderContext>()
        .saturating_add(16 * 1024)
        .saturating_add(block_size / 2)
        .saturating_add(estimate_match_state_bytes(ctx.cparams))
        .saturating_add(estimate_token_bytes(ctx.cparams))
        .saturating_add(estimate_ldm_bytes(ctx, block_size))
}

fn estimate_stream_buffers(cparams: ZSTD_compressionParameters) -> usize {
    let window_size = estimate_window_size(cparams);
    let block_size = estimate_block_size(cparams);
    window_size
        .saturating_add(block_size)
        .saturating_add(compress_bound(block_size))
        .saturating_add(1)
}

pub(crate) fn estimate_cctx_size_from_cparams(cparams: ZSTD_compressionParameters) -> usize {
    let mut ctx = EncoderContext::default();
    ctx.cparams = normalize_cparams(cparams);
    estimate_cctx_size_from_context(&ctx)
}

pub(crate) fn estimate_cctx_size_from_level(compression_level: c_int) -> usize {
    let start_level = core::cmp::min(compression_level, 1);
    let mut largest = 0;
    for level in start_level..=compression_level {
        let size = estimate_cctx_size_from_cparams(get_cparams(level, ZSTD_CONTENTSIZE_UNKNOWN, 0));
        largest = largest.max(size);
    }
    largest
}

pub(crate) fn cctx_params_size(params: *const ZSTD_CCtx_params) -> Result<usize, ZSTD_ErrorCode> {
    let ctx = crate::compress::cctx_params::context_from_cctx_params(params)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    if ctx.nb_workers >= 1 {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }
    Ok(estimate_cctx_size_from_context(&ctx))
}

pub(crate) fn estimate_cstream_size_from_cparams(cparams: ZSTD_compressionParameters) -> usize {
    estimate_cctx_size_from_cparams(cparams).saturating_add(estimate_stream_buffers(cparams))
}

pub(crate) fn estimate_cstream_size_from_level(compression_level: c_int) -> usize {
    let start_level = core::cmp::min(compression_level, 1);
    let mut largest = 0;
    for level in start_level..=compression_level {
        let size =
            estimate_cstream_size_from_cparams(get_cparams(level, ZSTD_CONTENTSIZE_UNKNOWN, 0));
        largest = largest.max(size);
    }
    largest
}

pub(crate) fn cstream_size_estimate(
    params: *const ZSTD_CCtx_params,
) -> Result<usize, ZSTD_ErrorCode> {
    let ctx = crate::compress::cctx_params::context_from_cctx_params(params)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_GENERIC)?;
    if ctx.nb_workers >= 1 {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }
    Ok(estimate_cctx_size_from_context(&ctx).saturating_add(estimate_stream_buffers(ctx.cparams)))
}

pub(crate) fn cdict_size_estimate(dict_size: usize) -> usize {
    alloc::base_size::<EncoderDictionary>() + alloc::heap_bytes(dict_size)
}

pub(crate) fn cdict_size_estimate_advanced(
    dict_size: usize,
    dict_load_method: ZSTD_dictLoadMethod_e,
) -> usize {
    alloc::base_size::<EncoderDictionary>()
        + match dict_load_method {
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy => alloc::heap_bytes(dict_size),
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef => 0,
        }
}

pub(crate) fn load_dictionary(
    ctx: &mut EncoderContext,
    dict: *const c_void,
    dict_size: usize,
    compression_level: c_int,
) -> Result<(), ZSTD_ErrorCode> {
    let dict =
        optional_src_slice(dict, dict_size).ok_or(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong)?;
    if dict.is_empty() {
        ctx.set_dict(None);
    } else {
        ctx.set_dict(Some(EncoderDictionary::from_bytes(
            dict,
            compression_level,
        )?));
    }
    Ok(())
}

pub(crate) fn load_dictionary_advanced(
    ctx: &mut EncoderContext,
    dict: *const c_void,
    dict_size: usize,
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
    compression_level: c_int,
) -> Result<(), ZSTD_ErrorCode> {
    let dict =
        optional_src_slice(dict, dict_size).ok_or(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong)?;
    if dict.is_empty() {
        ctx.set_dict(None);
    } else {
        ctx.set_dict(Some(EncoderDictionary::from_settings(
            dict,
            compression_level,
            get_cparams(compression_level, ZSTD_CONTENTSIZE_UNKNOWN, dict_size),
            ctx.enable_long_distance_matching,
            ctx.nb_workers,
            ctx.job_size,
            ctx.overlap_log,
            ctx.block_delimiters,
            ctx.validate_sequences,
            ctx.enable_seq_producer_fallback,
            dict_load_method,
            dict_content_type,
        )?));
    }
    Ok(())
}

pub(crate) fn validate_custom_mem(custom_mem: ZSTD_customMem) -> bool {
    custom_mem.customAlloc.is_none() && custom_mem.customFree.is_none()
}

pub(crate) fn to_result(code: Result<usize, ZSTD_ErrorCode>) -> usize {
    match code {
        Ok(size) => size,
        Err(error) => error_result(error),
    }
}
