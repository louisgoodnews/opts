//! Benchmark-only missing-texture sentinel (M-0054, `docs/src/architecture/scenery-benchmark.md`).
//!
//! When enabled, a material whose diffuse texture failed to load, or whose material slot does not
//! exist in the mesh, is drawn as **unlit magenta** `(255, 0, 255)` instead of falling back to the
//! mesh's diffuse colour or white. The scenery benchmark counts those pixels exactly. The game
//! never enables it.

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::color::Color;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// The sentinel colour in sRGB bytes.
pub const SENTINEL_RGB: [u8; 3] = [255, 0, 255];

/// Turns the sentinel on or off for every model loaded afterwards (process-wide).
pub fn set_missing_texture_sentinel(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Whether the sentinel is on.
#[must_use]
pub fn missing_texture_sentinel() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The unlit, opaque magenta material that replaces a fallback.
#[must_use]
pub fn sentinel_material() -> StandardMaterial {
    let [r, g, b] = SENTINEL_RGB;
    StandardMaterial {
        base_color: Color::srgb_u8(r, g, b),
        unlit: true,
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        ..Default::default()
    }
}
