/// Phase 3 owns the stable single-threaded compression ABI surface.
/// Multithreaded compression, CCtxParams, static workspaces, and sequence
/// producer APIs remain deferred to phase 4.
pub mod block;
pub mod cctx;
pub mod cdict;
pub mod cstream;
pub mod frame;
pub mod ldm;
pub mod literals;
pub mod match_state;
pub mod params;
pub mod sequences;

pub mod strategies {
    pub mod double_fast;
    pub mod fast;
    pub mod lazy;
    pub mod opt;
}
