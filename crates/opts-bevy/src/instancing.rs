//! Switch for the mesh-instancing prototype layers (M-0082,
//! `docs/src/architecture/instancing-research.md`).
//!
//! `OPTS_INSTANCING` selects the layer for the whole run (read once):
//!
//! | Value | Layer |
//! | :--- | :--- |
//! | unset, `off` | baseline: the loader as before |
//! | `dedupe` | (a) materials and meshes with equal content share one asset handle |
//! | `instance` | (a) plus (c) static scenery drawn by one manual instanced draw per (mesh, material) |
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
    /// De-duplication plus manual instancing of static scenery.
    Instance,
}

/// The mode from `OPTS_INSTANCING` (read once).
#[must_use]
pub fn mode() -> InstancingMode {
    static MODE: OnceLock<InstancingMode> = OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("OPTS_INSTANCING").as_deref() {
        Ok("dedupe") => InstancingMode::Dedupe,
        Ok("instance") => InstancingMode::Instance,
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
        InstancingMode::Instance => "instance",
    }
}
