//! Unit tests for `super` (`sky_cube.rs`): the policy, the sky texels and the sync system.

use bevy::prelude::*;

use super::*;
use crate::light_quality::LightQuality;
use crate::MainCamera;

mod policy;
mod sync;
mod texels;

fn input() -> SkyCubeInput {
    SkyCubeInput {
        horizon: [0.8, 0.85, 0.95],
        zenith: [0.2, 0.4, 0.8],
        sun_dir: Vec3::new(0.3, 0.6, 0.4).normalize(),
        sun_color: [1.0, 0.95, 0.8],
        sun_elevation_deg: 36.0,
        direct: 1.0,
        ambient: 1.0,
        fog: 0.0,
        revision: 1,
    }
}

fn snap(rev: u64, elev: f32, w: [f32; 3]) -> RegenSnapshot {
    RegenSnapshot {
        revision: rev,
        sun_elevation_deg: elev,
        weather: w,
    }
}
