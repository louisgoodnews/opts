//! The moon disc: its mesh and phase material, and its brightness by day and night.

use super::*;

/// Edge of the moon disc quad in metres (7.7° at the dome radius: artistic, the real moon is 0.5°).
pub const MOON_DISC_SIZE_M: f32 = 120.0;
/// Brightness of the disc in full daylight (it is additive, so by day it stays a faint pale disc).
pub const MOON_DISC_DAY_BRIGHTNESS: f32 = 0.35;

/// Moon discs still without a mesh.
type NewDiscsQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static MoonDiscTexture), (With<MoonDisc>, Without<Mesh3d>)>;

/// Gives the moon disc its mesh and phase material (needs the material plugin; without it the
/// disc stays invisible, as in headless tests).
pub fn setup_moon_disc(
    mut commands: Commands,
    discs: NewDiscsQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<MoonMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (entity, texture) in &discs {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Rectangle::new(MOON_DISC_SIZE_M, MOON_DISC_SIZE_M))),
            MeshMaterial3d(materials.add(MoonMaterial::new(texture.0.clone(), 0.0, 0.0, 1.0))),
            Transform::from_scale(Vec3::ZERO),
        ));
    }
}

/// Brightness of the moon disc: faint by day, full at night, dimmed by cloud.
#[must_use]
pub fn disc_brightness(sun_elevation_deg: f32, weather_moon: f32) -> f32 {
    let night = moon_night_factor(sun_elevation_deg);
    (MOON_DISC_DAY_BRIGHTNESS + (1.0 - MOON_DISC_DAY_BRIGHTNESS) * night) * weather_moon
}
