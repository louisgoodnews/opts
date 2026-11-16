//! Data-driven sky: dome texture, sun/moon discs and light colours from the installation's
//! `envir.cfg`, driven by the sun of the [`SessionEnvironment`] (date, time, weather).
//!
//! OMSI maps carry no geographic position, so the sun path assumes a fixed mid-latitude
//! (`opts_bevy::environment::LATITUDE_DEG`) and treats the clock time as local solar time.

use bevy::prelude::*;
use opts_bevy::ground::CpuImage;
use opts_bevy::SessionEnvironment;
use opts_core::formats::envir::EnvirConfig;

/// Radius of the sky dome; stays inside the default 1000 m far plane.
pub(crate) const DOME_RADIUS: f32 = 900.0;
/// Clock seconds between sky texture rebuilds (a jump of the environment rebuilds at once).
const REFRESH_S: f64 = 0.5;
/// Peak illuminance of full white direct sunlight.
const SUN_ILLUMINANCE: f32 = 20_000.0;
/// Ambient brightness for full white ambient light.
const AMBIENT_BRIGHTNESS: f32 = 700.0;

/// Sky state for the running session.
#[derive(Resource)]
pub struct Sky {
    pub(super) config: EnvirConfig,
    pub(super) textures: [CpuImage; 3],
    pub(super) image: Handle<Image>,
    /// Clock seconds of the last rebuild.
    pub(super) last_refresh: f64,
    /// Environment revision of the last rebuild (`None` before the first).
    pub(super) last_revision: Option<u64>,
    /// Whether the last rebuild wrote the clear colour for an HDR camera.
    pub(super) last_hdr: Option<bool>,
    /// The ground-fog sky clearing of the last rebuild, in 1/32 steps (the sky is rebuilt when
    /// the camera's height moves it by a step).
    pub(super) last_fog_clear: u8,
}

/// The sky's colour at the horizon (linear RGB `0..=1`), refreshed with the sky texture; the fog
/// takes its colour from it (`weather::fog`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SkyHorizon(pub [f32; 3]);

/// Marks the sky dome.
#[derive(Component)]
pub struct SkyDome;
/// Marks the sun light and disc.
#[derive(Component)]
pub struct SunLight;
/// Marks the sun disc quad.
#[derive(Component)]
pub struct SunDisc;
/// Marks the moon disc quad; `in_game::moon` gives it its mesh and material.
#[derive(Component)]
pub struct MoonDisc;
/// The moon photograph (`Texture/mond.bmp`) waiting for the disc's material.
#[derive(Component)]
pub struct MoonDiscTexture(pub Option<Handle<Image>>);

mod spawn;
pub use spawn::*;

mod geometry;
pub(crate) use geometry::direction;
pub use geometry::{place_sky_dome, sun_position};
mod texture;
use geometry::place_on_dome;
use texture::refresh_texture;
mod lights;
use lights::{clear_colour_of, set_ambient, set_clear_colour, sun_light};
mod update;
pub use update::{update_sky, SkyInputs, SkyQueries, SkyTargets};

#[cfg(test)]
mod tests;
