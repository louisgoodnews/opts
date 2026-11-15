//! Tuning of the map light pool and the full-intensity illuminance of a lamp.

use bevy::prelude::*;

/// Illuminance of a `[maplight]` at its full-intensity radius (lx). OMSI's own description:
/// full intensity within the radius, then falling off with the square of the distance. "Full"
/// is the scene's full lighting level, so it is set to half the midday sun of the session sky
/// (20,000 lx): a lamp 7 m above the road then lights it with about 1,300 lx, which reads as a
/// lit patch under the game's fixed exposure. The physical value of a street lamp (≈ 15 lx)
/// is invisible at that exposure.
pub const FULL_LUX: f32 = 10_000.0;

/// Tuning of the map light pool.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct MapLightSettings {
    /// Map lights are drawn at all.
    pub enabled: bool,
    /// Point lights in the pool.
    pub pool_size: usize,
    /// Only lamps within this distance of the camera get a light (m).
    pub radius_m: f32,
    /// A lamp that has a light keeps it while it is at most this much farther than the
    /// farthest newly chosen lamp (m).
    pub hysteresis_m: f32,
    /// Seconds between two assignments.
    pub interval_s: f32,
    /// Illuminance at the full-intensity radius (lx).
    pub full_lux: f32,
    /// Fade time in and out (s).
    pub fade_s: f32,
    /// While the camera is in an interior view of the player vehicle, lamps over the vehicle
    /// (inside its footprint prism, from the roof up) get no light (`lighting-system.md` 4.5).
    pub exclude_over_vehicle: bool,
    /// Factor on the pool's intensity while the camera is in an interior view, so a lamp outside
    /// does not out-shine the cabin light ([`crate::light_rooms::INTERIOR_VIEW_SCALE`]).
    pub interior_view_scale: f32,
}

impl Default for MapLightSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            pool_size: 48,
            radius_m: 200.0,
            hysteresis_m: 15.0,
            interval_s: 0.25,
            full_lux: FULL_LUX,
            fade_s: 0.5,
            exclude_over_vehicle: true,
            interior_view_scale: crate::light_rooms::INTERIOR_VIEW_SCALE,
        }
    }
}
