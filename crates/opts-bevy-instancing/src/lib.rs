//! Switch for the mesh-instancing prototype layers (M-0082,
//! `docs/src/architecture/instancing-research.md`).
//!
//! `OPTS_INSTANCING` selects the layer for the whole run (read once):
//!
//! | Value | Layer |
//! | :--- | :--- |
//! | unset, `off` | baseline: the loader as before |
//! | `dedupe` | (a) materials and meshes with equal content share one asset handle |
//! | `merge` | (a) plus (c) static scenery baked into one mesh per (material, 64 m cell, tile) |
//!
//! Layer (b), Bevy's automatic batching, is always on; the prototype only measures it.

use std::sync::OnceLock;

/// The active prototype layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstancingMode {
    /// The loader as before.
    Off,
    /// Equal materials and meshes share handles.
    Dedupe,
    /// De-duplication plus static merging: one mesh per (material, cell) and tile.
    Merge,
}

/// The mode from `OPTS_INSTANCING` (read once).
#[must_use]
pub fn mode() -> InstancingMode {
    static MODE: OnceLock<InstancingMode> = OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("OPTS_INSTANCING").as_deref() {
        Ok("dedupe") => InstancingMode::Dedupe,
        Ok("merge") => InstancingMode::Merge,
        _ => InstancingMode::Off,
    })
}

/// Whether handles are shared by content.
#[must_use]
pub fn dedupe() -> bool {
    mode() != InstancingMode::Off
}

/// Name of the active mode, for `render.stats`.
#[must_use]
pub fn mode_name() -> &'static str {
    match mode() {
        InstancingMode::Off => "off",
        InstancingMode::Dedupe => "dedupe",
        InstancingMode::Merge => "merge",
    }
}

mod bake;
mod keys;
mod merge;
mod pool;

pub mod render_stats;

pub use keys::*;
pub use merge::*;
pub use pool::*;

/// Whether static scenery is merged.
#[must_use]
pub fn merge() -> bool {
    mode() == InstancingMode::Merge
}

/// Edge of a merge cell in metres (`OPTS_MERGE_CELL`, default 64).
#[must_use]
pub fn merge_cell_m() -> f32 {
    static CELL: OnceLock<f32> = OnceLock::new();
    *CELL.get_or_init(|| {
        std::env::var("OPTS_MERGE_CELL")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|v: &f32| *v > 1.0)
            .unwrap_or(64.0)
    })
}
