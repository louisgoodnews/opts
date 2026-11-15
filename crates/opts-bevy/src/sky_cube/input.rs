//! What the game feeds the sky cube (`SkyCubeInput`) and the snapshot the policy compares.

use bevy::prelude::{Resource, Vec3};

/// What the game knows about the sky; all colours are scene-linear `0..=1`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct SkyCubeInput {
    /// Sky colour at the horizon (with the weather's veil).
    pub horizon: [f32; 3],
    /// Sky colour at the zenith.
    pub zenith: [f32; 3],
    /// Direction towards the sun (unit, Bevy space).
    pub sun_dir: Vec3,
    /// Colour of the sun light.
    pub sun_color: [f32; 3],
    /// Elevation of the sun in degrees (the spot exists only above 0).
    pub sun_elevation_deg: f32,
    /// The weather's attenuation of the direct light, `0..=1`.
    pub direct: f32,
    /// The weather's attenuation of the ambient light, `0..=1`.
    pub ambient: f32,
    /// The fog share of the weather, `0..=1` (0 clear).
    pub fog: f32,
    /// The environment revision (a jump of date or weather).
    pub revision: u64,
}

impl SkyCubeInput {
    /// The values the regeneration policy compares.
    #[must_use]
    pub fn snapshot(&self) -> RegenSnapshot {
        RegenSnapshot {
            revision: self.revision,
            sun_elevation_deg: self.sun_elevation_deg,
            weather: [self.direct, self.ambient, self.fog],
        }
    }
}

/// What the cube was last built from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegenSnapshot {
    /// Environment revision.
    pub revision: u64,
    /// Sun elevation (degrees).
    pub sun_elevation_deg: f32,
    /// Direct attenuation, ambient attenuation, fog share.
    pub weather: [f32; 3],
}
