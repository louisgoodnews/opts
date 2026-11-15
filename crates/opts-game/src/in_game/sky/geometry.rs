//! Sun path and dome placement: the sun position of the local hour and the dome turned to the sun.

use bevy::prelude::*;

use super::*;

/// Sun elevation and azimuth (degrees; azimuth clockwise from north) for the local solar hour:
/// [`opts_bevy::environment::sun_position`] at the sky's latitude.
#[must_use]
pub fn sun_position(hour: f32, day_of_year: f32) -> (f32, f32) {
    opts_bevy::environment::sun_position(hour, day_of_year, opts_bevy::environment::LATITUDE_DEG)
}

/// Direction towards a point at the given elevation/azimuth, in Bevy space (north = −Z).
pub(crate) fn direction(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
    let (el, az) = (elevation_deg.to_radians(), azimuth_deg.to_radians());
    Vec3::new(el.cos() * az.sin(), el.sin(), -el.cos() * az.cos())
}

/// Moves the sky dome to the [`MainCamera`](opts_bevy::MainCamera) and turns it so that texture
/// `u = 0` faces the sun (the dusk texture's glow is authored around it).
pub fn place_sky_dome(
    env: Res<SessionEnvironment>,
    cameras: Query<&GlobalTransform, (With<opts_bevy::MainCamera>, Without<SkyDome>)>,
    mut dome: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    for mut t in &mut dome {
        t.translation = cam;
        t.rotation = Quat::from_rotation_y(-env.sun_az_deg().to_radians());
    }
}

/// The sun disc: on the dome in the sun's direction, hidden below the horizon.
pub(super) fn place_on_dome(cam: Vec3, dir: Vec3) -> Transform {
    let mut t = Transform::from_translation(cam + dir * (DOME_RADIUS - 10.0));
    t.look_at(cam, Vec3::Y);
    if dir.y <= -0.05 {
        t.scale = Vec3::ZERO;
    }
    t
}
