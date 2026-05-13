pub const BASELINE_EXPORT_COUNT: usize = 185;
pub const EXPORT_MAP_PATH: &str = "abi/export_map.toml";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhaseAnchor {
    pub symbol: &'static str,
    pub owning_phase: u8,
    pub owner_module: &'static str,
}

pub const PHASE_ANCHORS: &[PhaseAnchor] = &[
    PhaseAnchor {
        symbol: "ZSTD_decompress",
        owning_phase: 2,
        owner_module: "crate::decompress::dctx",
    },
    PhaseAnchor {
        symbol: "ZSTD_decompressDCtx",
        owning_phase: 2,
        owner_module: "crate::decompress::dctx",
    },
    PhaseAnchor {
        symbol: "ZSTD_DCtx_reset",
        owning_phase: 2,
        owner_module: "crate::decompress::dctx",
    },
    PhaseAnchor {
        symbol: "ZSTD_getDictID_fromFrame",
        owning_phase: 2,
        owner_module: "crate::decompress::frame",
    },
    PhaseAnchor {
        symbol: "ZSTD_compressBound",
        owning_phase: 3,
        owner_module: "crate::compress::cctx",
    },
    PhaseAnchor {
        symbol: "ZSTD_createThreadPool",
        owning_phase: 4,
        owner_module: "crate::threading::pool",
    },
    PhaseAnchor {
        symbol: "ZSTD_freeThreadPool",
        owning_phase: 4,
        owner_module: "crate::threading::pool",
    },
    PhaseAnchor {
        symbol: "ZSTD_CCtx_refThreadPool",
        owning_phase: 4,
        owner_module: "crate::threading::pool",
    },
];
