/// Phase 4 completes the public compression ABI surface, including advanced
/// parameter, sequence, static-workspace, and multithreaded entry points.
pub mod advanced;
pub mod block;
pub mod cctx;
pub mod cctx_params;
pub mod cdict;
pub mod compat;
pub mod cstream;
pub mod frame;
pub mod ldm;
pub mod literals;
pub mod match_state;
pub mod params;
pub mod sequence_api;
pub mod sequences;
pub mod static_ctx;

pub mod strategies {
    pub mod double_fast;
    pub mod fast;
    pub mod lazy;
    pub mod opt;
}
