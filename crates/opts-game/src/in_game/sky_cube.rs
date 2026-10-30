//! Feeds the shared sky cube map ([`opts_bevy::sky_cube`]) from the session's sky.
//!
//! The dome texture that `sky.rs` rebuilds (with the weather's fog veil already in it) is the
//! source of the colours: its first row is the zenith, its last row the horizon. The sun comes
//! from the session environment and the sun light, the weather from
//! [`SessionEnvironment::light_attenuation`] and the visibility. Nothing here touches the sky
//! itself.

use bevy::prelude::*;
use opts_bevy::sky_cube::SkyCubeInput;
use opts_bevy::weather::FOG_MAX_VISIBILITY_M;
use opts_bevy::SessionEnvironment;

use crate::in_game::sky::{direction, SkyDome, SunLight};

/// The sRGB byte `b` as a scene-linear value.
fn linear(b: f32) -> f32 {
    let c = b / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Mean scene-linear colour of row `y` of an `Rgba8` image of `width` pixels; `None` when the
/// image data is too short.
#[must_use]
pub fn row_colour(data: &[u8], width: usize, y: usize) -> Option<[f32; 3]> {
    let start = y.checked_mul(width)?.checked_mul(4)?;
    let row = data.get(start..start.checked_add(width.checked_mul(4)?)?)?;
    if width == 0 {
        return None;
    }
    let mut sum = [0.0f32; 3];
    for px in row.chunks_exact(4) {
        for (c, s) in sum.iter_mut().enumerate() {
            *s += linear(f32::from(px[c]));
        }
    }
    Some(sum.map(|s| s / width as f32))
}

/// The sky dome's material handle query.
type DomeQuery<'w, 's> =
    Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>, With<SkyDome>>;
/// The sun's directional light query.
type SunQuery<'w, 's> = Query<'w, 's, &'static DirectionalLight, With<SunLight>>;

/// Writes [`SkyCubeInput`] from the dome texture, the sun and the weather.
pub fn feed_sky_cube(
    env: Res<SessionEnvironment>,
    dome: DomeQuery,
    materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    sun: SunQuery,
    mut commands: Commands,
    current: Option<ResMut<SkyCubeInput>>,
) {
    let Some(texture) = dome
        .iter()
        .next()
        .and_then(|m| materials.get(&m.0))
        .and_then(|m| m.base_color_texture.as_ref())
        .and_then(|h| images.get(h))
    else {
        return;
    };
    let Some(data) = texture.data.as_deref() else {
        return;
    };
    let (w, h) = (
        texture.texture_descriptor.size.width as usize,
        texture.texture_descriptor.size.height as usize,
    );
    let (Some(zenith), Some(horizon)) = (
        row_colour(data, w, 0),
        row_colour(data, w, h.saturating_sub(1)),
    ) else {
        return;
    };
    // The texture is black until the sky has built it once.
    if zenith.iter().chain(&horizon).all(|c| *c == 0.0) {
        return;
    }
    let elevation = env.sun_alt_deg();
    let la = env.light_attenuation();
    let sun_color = sun.iter().next().map_or([1.0; 3], |l| {
        let c = l.color.to_linear();
        [c.red, c.green, c.blue]
    });
    let visibility = env.conditions().fog_visibility_m;
    let input = SkyCubeInput {
        horizon,
        zenith,
        sun_dir: direction(elevation, env.sun_az_deg()),
        sun_color,
        sun_elevation_deg: elevation,
        direct: la.direct,
        ambient: la.ambient,
        fog: (1.0 - visibility / FOG_MAX_VISIBILITY_M).clamp(0.0, 1.0),
        revision: env.revision(),
    };
    match current {
        Some(mut c) => {
            if *c != input {
                *c = input;
            }
        }
        None => commands.insert_resource(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_average_in_linear_light() {
        // Two pixels: black and white; the mean of sRGB 0 and 255 is linear 0.5, not 0.21.
        let data = [0, 0, 0, 255, 255, 255, 255, 255];
        let c = row_colour(&data, 2, 0).unwrap();
        assert!((c[0] - 0.5).abs() < 1e-5 && (c[2] - 0.5).abs() < 1e-5);
        assert!(row_colour(&data, 2, 1).is_none(), "past the end");
        assert!(row_colour(&data, 0, 0).is_none());
    }
}
