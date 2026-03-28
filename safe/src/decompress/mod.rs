/// Phase 2 owns the stable decompression ABI surface.
/// Advanced decompression, static workspace, and estimate helpers remain deferred
/// to phase 4 and stay recorded as such in `safe/abi/export_map.toml`.
pub mod block;
pub mod dctx;
pub mod ddict;
pub mod dstream;
pub mod frame;
pub mod fse;
pub mod huf;
pub mod legacy;
