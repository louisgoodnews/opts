//! Wave displacement of the water surface (M-0025 Phase B): two travelling sine trains, one along
//! east and one along north, scaled by the water's depth so the surface stays at its level where
//! the water meets the shore.
//!
//! The parameters are not in the design (`docs/src/architecture/terrain-rendering.md` section 7
//! says the surface has none); these are a gentle first choice, to be checked in the game.
//! Nothing calls [`displaced`] yet: wiring it needs a per-frame update of the water meshes (or a
//! shader), and a look in the game.

/// Amplitude (m) and wavelength (m) of the train that runs along east.
const WAVE_X_M: f32 = 0.06;
const WAVE_X_LEN_M: f32 = 9.0;
/// Amplitude (m) and wavelength (m) of the train that runs along north (Bevy `z`).
const WAVE_Z_M: f32 = 0.04;
const WAVE_Z_LEN_M: f32 = 6.5;
/// Periods (s) of the two trains.
const WAVE_X_PERIOD_S: f32 = 4.0;
const WAVE_Z_PERIOD_S: f32 = 3.0;
/// Depth (m) at which the waves reach their full amplitude; at the shore they are zero.
pub const WAVE_FULL_DEPTH_M: f32 = 1.0;

/// Vertical offset (m) of the surface at Bevy `(x, z)` at time `t` (s), before the depth scale.
/// Both trains move towards increasing `x` and `z`.
#[must_use]
pub fn wave_offset(x: f32, z: f32, t: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    WAVE_X_M * (tau * (x / WAVE_X_LEN_M - t / WAVE_X_PERIOD_S)).sin()
        + WAVE_Z_M * (tau * (z / WAVE_Z_LEN_M - t / WAVE_Z_PERIOD_S)).sin()
}

/// Height (m) of the displaced surface at `(x, z)` for water at `level` with `depth` metres of
/// water above the ground: the wave offset scaled to `depth / WAVE_FULL_DEPTH_M`, at most 1.
#[must_use]
pub fn displaced_height(level: f32, depth: f32, x: f32, z: f32, t: f32) -> f32 {
    let scale = (depth / WAVE_FULL_DEPTH_M).clamp(0.0, 1.0);
    level + scale * wave_offset(x, z, t)
}

/// The rest positions of a water surface moved to time `t`: `depths[k]` is the depth of vertex
/// `k` (the vertex's `y` is its level). Vertices without a depth stay where they are.
#[must_use]
pub fn displaced(rest: &[[f32; 3]], depths: &[f32], t: f32) -> Vec<[f32; 3]> {
    rest.iter()
        .enumerate()
        .map(|(k, p)| {
            let depth = depths.get(k).copied().unwrap_or(0.0);
            [p[0], displaced_height(p[1], depth, p[0], p[2], t), p[2]]
        })
        .collect()
}
