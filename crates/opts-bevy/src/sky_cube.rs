//! The shared sky cube map for glass reflections (`docs/src/architecture/reflections-research.md`
//! section 7 step B, `lighting-system.md` section 9).
//!
//! * The game writes [`SkyCubeInput`] (horizon and zenith colours of the session's sky, the sun,
//!   the weather); this module owns **one** cube image and, while
//!   [`LightQuality::glass_reflections`] is on, a [`GeneratedEnvironmentMapLight`] on the
//!   [`MainCamera`] that points at it. Bevy filters the cube into diffuse and specular maps every
//!   frame (that is the 1.2 ms of the budget; the source is read, nothing is cached), so the
//!   regeneration of the *source* below is cheap CPU work, not GPU work.
//! * [`should_regenerate`] is the policy: at most once per [`REGEN_MIN_INTERVAL_S`] real seconds,
//!   and only when the environment revision changed, the sun moved by more than
//!   [`REGEN_SUN_DELTA_DEG`], or the weather moved by more than [`WEATHER_EPS`]. The first cube is
//!   built at once.
//! * The intensity follows [`exposure`] every frame (no regeneration): daylight times the
//!   weather's ambient attenuation, so night does not glow.
//! * With `glass_reflections` off the components are removed and the image freed: the output is
//!   today's.
//!
//! Mirror cameras do not get the light (a second filter pass costs about 1 ms per camera).

use bevy::asset::RenderAssetUsages;
use bevy::light::{EnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::light_quality::{LightQuality, LightQualitySet};
use crate::MainCamera;

/// Edge of the source cube in texels (a power of two; Bevy filters it into its own maps).
pub const CUBE_SIZE: u32 = 64;
/// Intensity (cd/m2) at full daylight and clear sky. The cube's diffuse part also lights every
/// standard material, so this is a compromise between visible glass reflections and the extra
/// ambient light in shadow (the research prototype saw a washed-out image at 3000).
pub const BASE_INTENSITY: f32 = 800.0;
/// Shortest time between two regenerations of the cube (real seconds).
pub const REGEN_MIN_INTERVAL_S: f64 = 2.0;
/// The sun must have moved by more than this (degrees of elevation) to regenerate.
pub const REGEN_SUN_DELTA_DEG: f32 = 2.0;
/// A weather value (attenuations, fog share) must have moved by more than this to regenerate.
pub const WEATHER_EPS: f32 = 0.02;
/// Radiance of the sun's disc at full direct light, relative to the sky colours (which are
/// `0..=1`). The disc is a small, very bright spot in the specular reflection.
pub const SUN_SPOT_RADIANCE: f32 = 20.0;
/// Sharpness of the sun spot (`cos^n`).
pub const SUN_SPOT_POWER: f32 = 400.0;
/// Share of the horizon colour the ground below the horizon reflects.
pub const GROUND_SHARE: f32 = 0.3;
/// Least [`exposure`] at night (a trace of moonlight; the sky texture is dark then, too).
pub const NIGHT_EXPOSURE: f32 = 0.03;

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

/// Whether the cube must be rebuilt: `last` is the snapshot and real time (seconds) of the last
/// build, `None` before the first.
#[must_use]
pub fn should_regenerate(
    last: Option<(&RegenSnapshot, f64)>,
    now: &RegenSnapshot,
    now_s: f64,
) -> bool {
    let Some((prev, at)) = last else { return true };
    if now_s - at < REGEN_MIN_INTERVAL_S {
        return false;
    }
    prev.revision != now.revision
        || (prev.sun_elevation_deg - now.sun_elevation_deg).abs() > REGEN_SUN_DELTA_DEG
        || prev
            .weather
            .iter()
            .zip(now.weather)
            .any(|(a, b)| (a - b).abs() > WEATHER_EPS)
}

/// How much of [`BASE_INTENSITY`] is used: zero in the dark (a trace of moonlight), full in the
/// day (smoothstep over sun elevation -4 to 10 degrees), scaled by the weather's ambient
/// attenuation.
#[must_use]
pub fn exposure(sun_elevation_deg: f32, ambient_attenuation: f32) -> f32 {
    let t = ((sun_elevation_deg + 4.0) / 14.0).clamp(0.0, 1.0);
    let day = t * t * (3.0 - 2.0 * t);
    (NIGHT_EXPOSURE + (1.0 - NIGHT_EXPOSURE) * day) * ambient_attenuation.clamp(0.0, 1.0)
}

/// The radiance the sky has in direction `d` (unit).
#[must_use]
pub fn sky_radiance(d: Vec3, input: &SkyCubeInput) -> Vec3 {
    let horizon = Vec3::from(input.horizon);
    let zenith = Vec3::from(input.zenith);
    let up = d.y;
    let mut c = if up >= 0.0 {
        horizon.lerp(zenith, up.powf(0.5))
    } else {
        horizon.lerp(horizon * GROUND_SHARE, (-up * 6.0).min(1.0))
    };
    if input.sun_elevation_deg > 0.0 {
        c += Vec3::from(input.sun_color)
            * (SUN_SPOT_RADIANCE * input.direct)
            * d.dot(input.sun_dir).max(0.0).powf(SUN_SPOT_POWER);
    }
    c
}

/// Direction of the texel `(x, y)` on cube face `face` (Bevy's face order +X -X +Y -Y +Z -Z).
#[must_use]
pub fn face_direction(face: u32, x: u32, y: u32, size: u32) -> Vec3 {
    let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
    match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    }
    .normalize()
}

/// The texel data (`Rgba16Float`, six faces after each other) of the cube for `input`.
#[must_use]
pub fn cube_data(input: &SkyCubeInput, size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 6 * 8) as usize);
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let c = sky_radiance(face_direction(face, x, y, size), input);
                for ch in [c.x, c.y, c.z, 1.0] {
                    data.extend_from_slice(&crate::reflections::f16_bits(ch).to_le_bytes());
                }
            }
        }
    }
    data
}

fn cube_image(input: &SkyCubeInput, size: u32) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        cube_data(input, size),
        TextureFormat::Rgba16Float,
        // The main world keeps the data so the cube can be rewritten in place.
        RenderAssetUsages::default(),
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image
}

/// The one shared cube and when it was built.
#[derive(Resource, Default)]
pub struct SkyCubeState {
    /// The cube image while reflections are on.
    pub handle: Option<Handle<Image>>,
    last: Option<(RegenSnapshot, f64)>,
    /// How often the cube was built since start-up.
    pub builds: u32,
    /// The intensity last written to the camera.
    pub intensity: f32,
}

/// Marks a camera that carries the sky cube light.
#[derive(Component)]
pub struct SkyCubeApplied;

/// Keeps the cube and the camera light in step with the quality and the input.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_sky_cube(
    time: Res<Time<Real>>,
    quality: Res<LightQuality>,
    input: Option<Res<SkyCubeInput>>,
    cabin: Option<Res<crate::light_rooms::CabinView>>,
    mut state: ResMut<SkyCubeState>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
    mut cameras: Query<
        (
            Entity,
            Has<SkyCubeApplied>,
            Option<&mut GeneratedEnvironmentMapLight>,
            Option<&mut EnvironmentMapLight>,
        ),
        With<MainCamera>,
    >,
) {
    let input = input.filter(|_| quality.glass_reflections);
    let Some(input) = input else {
        // Off (or no sky yet): back to today's output, and free the cube.
        for (entity, applied, ..) in &cameras {
            if applied {
                commands.entity(entity).remove::<(
                    SkyCubeApplied,
                    GeneratedEnvironmentMapLight,
                    EnvironmentMapLight,
                )>();
            }
        }
        if state.handle.take().is_some() {
            state.last = None;
        }
        return;
    };
    let now = input.snapshot();
    let now_s = time.elapsed_secs_f64();
    let last = state.last.as_ref().map(|(s, t)| (s, *t));
    let handle = match state.handle.clone() {
        Some(h) if images.contains(&h) => {
            if should_regenerate(last, &now, now_s) {
                if let Some(mut img) = images.get_mut(&h) {
                    img.data = Some(cube_data(&input, CUBE_SIZE));
                }
                state.last = Some((now, now_s));
                state.builds += 1;
            }
            h
        }
        _ => {
            let h = images.add(cube_image(&input, CUBE_SIZE));
            state.handle = Some(h.clone());
            state.last = Some((now, now_s));
            state.builds += 1;
            h
        }
    };
    let interior = cabin.is_some_and(|c| c.interior_view);
    let intensity = BASE_INTENSITY
        * exposure(input.sun_elevation_deg, input.ambient)
        * if interior {
            crate::light_rooms::INTERIOR_AMBIENT_SCALE
        } else {
            1.0
        };
    state.intensity = intensity;
    for (entity, applied, generated, probe) in &mut cameras {
        if !applied {
            commands.entity(entity).insert((
                SkyCubeApplied,
                GeneratedEnvironmentMapLight {
                    environment_map: handle.clone(),
                    intensity,
                    ..default()
                },
            ));
            continue;
        }
        if let Some(mut g) = generated {
            if (g.intensity - intensity).abs() > 0.5 {
                g.intensity = intensity;
            }
        }
        if let Some(mut p) = probe {
            if (p.intensity - intensity).abs() > 0.5 {
                p.intensity = intensity;
            }
        }
    }
}

/// Registers [`sync_sky_cube`].
#[derive(Default)]
pub struct SkyCubePlugin;

impl Plugin for SkyCubePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .init_resource::<SkyCubeState>()
            .add_systems(
                Update,
                sync_sky_cube
                    .after(LightQualitySet)
                    .run_if(resource_exists::<Assets<Image>>),
            );
    }
}

#[cfg(test)]
mod tests;
