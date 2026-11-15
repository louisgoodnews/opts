//! The cloud layer of `[clouds]`: one disc at the base height that follows the camera, textured
//! with the cloud type's texture from `clouds.cfg`, scrolling downwind. A scattered type
//! (`sct`) shows its texture's gaps, an overcast one (`ovc`) is closed. The disc fades out at its
//! rim; the texture is fixed to the world, so driving does not slide the clouds.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::math::Affine2;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use opts_bevy::weather::{cloud_drift, weather_look};
use opts_bevy::WeatherCatalogue;

use crate::in_game::types::{InGameCamera, InGameEntity};

/// Radius of the disc; inside the camera's far plane.
pub const RADIUS_M: f32 = 950.0;
const SEGMENTS: usize = 48;
const RINGS: usize = 6;
/// The reference height follows the camera's height with this rate (1/s), so hills do not pass
/// through the layer.
const HEIGHT_FOLLOW: f32 = 0.3;

/// Marks the cloud disc.
#[derive(Component, Debug, Clone, Copy)]
pub struct CloudPlane;

/// State of the cloud layer.
#[derive(Resource, Default)]
pub struct CloudState {
    /// Loaded textures by file name (`None`: could not be read, do not try again).
    textures: HashMap<String, Option<Handle<Image>>>,
    mesh: Option<Handle<Mesh>>,
    /// Height the layer is measured from.
    reference_y: Option<f32>,
    /// The texture the material has.
    shown: Option<String>,
    /// The tint written to the material last.
    tint: f32,
}

/// UV transform of the disc: the texture is fixed to the world and carried along by the wind.
/// `camera_xz` is the disc's position, `drift` the cloud speed in m/s, `t` seconds, `size_m` the
/// texture repeat.
#[must_use]
pub fn uv_transform(camera_xz: Vec2, drift: Vec2, t: f64, size_m: f32) -> Affine2 {
    let size = f64::from(size_m.max(1.0));
    let off =
        |cam: f32, d: f32| ((f64::from(cam) - f64::from(d) * t) / size).rem_euclid(1.0) as f32;
    Affine2::from_scale_angle_translation(
        Vec2::splat(1.0 / size_m.max(1.0)),
        0.0,
        Vec2::new(off(camera_xz.x, drift.x), off(camera_xz.y, drift.y)),
    )
}

mod mesh;
use mesh::disc_mesh;
mod spawn;
pub use spawn::CloudInputs;
use spawn::{load_texture, spawn_plane};
mod update;
pub use update::update_clouds;

#[cfg(test)]
mod tests;
