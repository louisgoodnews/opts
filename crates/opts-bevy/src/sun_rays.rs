//! Sun and moon god rays with Bevy's `VolumetricFog`
//! (`docs/src/architecture/lighting-system.md` section 6 and 9).
//!
//! One system, [`sync_light_shafts`], makes the volumetric components follow
//! [`LightQuality`] and the shadow-carrying light:
//!
//! * the **carrier** is the directional light with a [`LightRole`] that has
//!   `shadow_maps_enabled` and light (the game's carrier system keeps exactly one such light and
//!   applies the 15 % hysteresis; this module only follows it, so a hand-over does not flicker);
//! * with `shafts != Off` and a carrier (the moon only with `night_shafts`): [`VolumetricFog`]
//!   (`step_count` from the quality, jitter on, `ambient_intensity` 0) goes on every
//!   [`MainCamera`], [`VolumetricLight`] on the carrier only, and one camera-following
//!   [`FogVolume`] cube ([`SHAFT_VOLUME_SIDE_M`]) exists;
//! * otherwise everything is removed and the volume despawned (Bevy then skips the whole pass:
//!   zero cost).
//!
//! It works with and without `Hdr` on the camera: without `Hdr` the materials tonemap in their own
//! shader and the fog adds linear light to an already tonemapped image, so the shaft intensity is
//! scaled by [`LDR_SHAFT_SCALE`].

use bevy::camera::Hdr;
use bevy::light::{FogVolume, VolumetricFog, VolumetricLight};
use bevy::prelude::*;

use crate::environment::SessionEnvironment;
use crate::light_quality::{LightQuality, LightQualitySet, Shafts};
use crate::MainCamera;

/// Side of the camera-following fog cube in metres: twice the shadow cascades' maximum distance
/// (150 m would be the reach; 200 m keeps the ray steps short, the camera is always inside).
pub const SHAFT_VOLUME_SIDE_M: f32 = 200.0;
/// Baseline haze `density_factor` per metre (`HAZE_BASE`): a clear sky has an extinction of about
/// 1e-4 per metre, far too thin to show a shaft inside the shadow range, so this is an artistic
/// haze ❓.
pub const HAZE_BASE: f32 = 0.0006;
/// Factor from the weather's Koschmieder extinction `3.912 / visibility` (per metre) to the
/// volume's `density_factor`. Bevy's extinction is `density_factor × (absorption + scattering)`
/// = `0.6 ×` with the defaults, so 1.0 makes the haze a little thinner than the distance fog.
pub const FOG_TO_SHAFT: f32 = 1.0;
/// Largest `density_factor`: thicker volumes wash the screen out and end every ray early. The
/// distance fog already is the fog; the haze volume only adds the light shafts and its
/// in-scattering takes the *sun's* colour, so a thick one (0.05) tinted the whole frame yellow at
/// a low sun. At 0.008 (extinction 0.005 per metre, 40 % in 100 m) the shafts of a foggy morning
/// stay and the tint is a hint.
pub const HAZE_MAX: f32 = 0.008;
/// `scattering_asymmetry`: strong forward scattering, shafts show when looking towards the light.
pub const SHAFT_ASYMMETRY: f32 = 0.65;
/// Overall factor of the volume's `light_intensity` (`gain`).
pub const SHAFT_GAIN: f32 = 0.9;
/// `VolumetricFog::ambient_intensity` (`ambient`): 0, the default 0.1 veils the whole screen.
pub const SHAFT_AMBIENT: f32 = 0.0;
/// The volume's `absorption` coefficient (`absorption`).
pub const SHAFT_ABSORPTION: f32 = 0.3;
/// The volume's `scattering` coefficient (`scatter`).
pub const SHAFT_SCATTERING: f32 = 0.3;
/// Name of the developer override variable, see [`ShaftStyle::parse_tune`].
pub const TUNE_ENV: &str = "OPTS_SHAFT_TUNE";
/// Brighter shafts for the moon: its light is about 20 times weaker than the sun's in game units,
/// so with the same intensity its beams would be invisible ❓ (tunable).
pub const MOON_SHAFT_GAIN: f32 = 2.0;
/// Intensity scale without `Hdr` on the camera (the fog adds untonemapped light, colours clip).
pub const LDR_SHAFT_SCALE: f32 = 0.6;
/// `VolumetricFog::jitter` in metres (no effect without temporal anti-aliasing, kept on as in the
/// design).
pub const SHAFT_JITTER: f32 = 0.5;

/// The look of the shafts: the constants above as a resource, overridable for tuning with
/// `OPTS_SHAFT_TUNE="haze=0.001,asym=0.6,gain=1,ambient=0,absorption=0.3,scatter=0.3,moon_gain=4"`
/// (any subset, read once at startup; see `docs/src/reference/developer-switches.md`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ShaftStyle {
    /// Baseline haze `density_factor` per metre (`haze`).
    pub haze: f32,
    /// Henyey-Greenstein asymmetry `g` (`asym`).
    pub asym: f32,
    /// Factor of the volume's `light_intensity` (`gain`).
    pub gain: f32,
    /// `VolumetricFog::ambient_intensity` (`ambient`).
    pub ambient: f32,
    /// Absorption coefficient (`absorption`).
    pub absorption: f32,
    /// Scattering coefficient (`scatter`).
    pub scatter: f32,
    /// Extra gain of the moon's shafts (`moon_gain`).
    pub moon_gain: f32,
}

impl Default for ShaftStyle {
    fn default() -> Self {
        Self {
            haze: HAZE_BASE,
            asym: SHAFT_ASYMMETRY,
            gain: SHAFT_GAIN,
            ambient: SHAFT_AMBIENT,
            absorption: SHAFT_ABSORPTION,
            scatter: SHAFT_SCATTERING,
            moon_gain: MOON_SHAFT_GAIN,
        }
    }
}

impl ShaftStyle {
    /// The defaults with the `key=value` pairs of `text` applied (comma separated, unknown keys
    /// and unparsable or negative values ignored; `asym` is clamped below 1).
    #[must_use]
    pub fn parse_tune(text: &str) -> Self {
        let mut st = Self::default();
        for pair in text.split(',') {
            let Some((k, v)) = pair.split_once('=') else {
                continue;
            };
            let Ok(v) = v.trim().parse::<f32>() else {
                continue;
            };
            if !v.is_finite() || v < 0.0 {
                continue;
            }
            match k.trim() {
                "haze" => st.haze = v,
                "asym" => st.asym = v.min(0.99),
                "gain" => st.gain = v,
                "ambient" => st.ambient = v,
                "absorption" => st.absorption = v,
                "scatter" => st.scatter = v,
                "moon_gain" => st.moon_gain = v,
                _ => {}
            }
        }
        st
    }

    /// The style from [`TUNE_ENV`] (the defaults when unset).
    #[must_use]
    pub fn from_env() -> Self {
        std::env::var(TUNE_ENV).map_or_else(|_| Self::default(), |t| Self::parse_tune(&t))
    }
}

/// Which of the two sky lights an entity is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightRole {
    /// The sun (also the fallback light when there is no `envir.cfg`).
    Sun,
    /// The moon.
    Moon,
}

/// Marks the one camera-following fog volume.
#[derive(Component, Debug, Clone, Copy)]
pub struct ShaftFogVolume;

/// The set of the shaft sync; the game's shadow-carrier system runs `.before` it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SunRaysSet;

/// The `density_factor` of the haze for a visibility in metres: the larger of the baseline
/// [`HAZE_BASE`] and the weather's extinction (`β = 3.912 / V`, Koschmieder) times
/// [`FOG_TO_SHAFT`], at most [`HAZE_MAX`]. Fog thickens the haze; clear air keeps the baseline.
#[must_use]
pub fn haze_density(visibility_m: f32, style: &ShaftStyle) -> f32 {
    let beta = 3.912 / visibility_m.max(1.0);
    (beta * FOG_TO_SHAFT).clamp(style.haze, HAZE_MAX.max(style.haze))
}

/// The `light_intensity` of the volume: the weather's shaft attenuation, the moon's gain and the
/// LDR scale.
#[must_use]
pub fn shaft_intensity(
    role: LightRole,
    shafts_attenuation: f32,
    hdr: bool,
    style: &ShaftStyle,
) -> f32 {
    let gain = match role {
        LightRole::Sun => style.gain,
        LightRole::Moon => style.gain * style.moon_gain,
    };
    let ldr = if hdr { 1.0 } else { LDR_SHAFT_SCALE };
    shafts_attenuation.clamp(0.0, 1.0) * gain * ldr
}

/// The index of the light that gets [`VolumetricLight`], if any: the first light with shadows and
/// light, of a role the quality allows, when shafts are on.
#[must_use]
pub fn pick_carrier(
    shafts: Shafts,
    night_shafts: bool,
    lights: impl IntoIterator<Item = (LightRole, bool, f32)>,
) -> Option<usize> {
    if shafts == Shafts::Off {
        return None;
    }
    lights.into_iter().position(|(role, shadows, illuminance)| {
        shadows && illuminance > 0.0 && (role == LightRole::Sun || night_shafts)
    })
}

type CameraRayItem<'a> = (
    Entity,
    &'a GlobalTransform,
    Option<&'a mut VolumetricFog>,
    Has<Hdr>,
    Option<&'a Camera>,
);

fn sync_camera_volumetric_fog(
    cameras: &mut Query<CameraRayItem<'_>, With<MainCamera>>,
    active: bool,
    quality: &LightQuality,
    style: &ShaftStyle,
    commands: &mut Commands,
) {
    for (entity, _, fog, _, _) in cameras.iter_mut() {
        match (active, fog) {
            (true, None) => {
                commands.entity(entity).insert(VolumetricFog {
                    ambient_intensity: style.ambient,
                    jitter: SHAFT_JITTER,
                    step_count: quality.shafts.steps(),
                    ..default()
                });
            }
            (true, Some(mut fog)) => {
                if fog.step_count != quality.shafts.steps() {
                    fog.step_count = quality.shafts.steps();
                }
                if fog.ambient_intensity != style.ambient {
                    fog.ambient_intensity = style.ambient;
                }
            }
            (false, Some(_)) => {
                commands.entity(entity).remove::<VolumetricFog>();
            }
            (false, None) => {}
        }
    }
}

fn sync_fog_volume_entities(
    volumes: &mut Query<(Entity, &mut Transform, &mut FogVolume), With<ShaftFogVolume>>,
    cam_pos: Vec3,
    density: f32,
    intensity: f32,
    style: &ShaftStyle,
    commands: &mut Commands,
) {
    let mut seen = false;
    for (entity, mut t, mut volume) in volumes.iter_mut() {
        if seen {
            commands.entity(entity).despawn();
            continue;
        }
        seen = true;
        if t.translation != cam_pos {
            t.translation = cam_pos;
        }
        if (volume.density_factor - density).abs() > 1e-6 {
            volume.density_factor = density;
        }
        if (volume.light_intensity - intensity).abs() > 1e-6 {
            volume.light_intensity = intensity;
        }
        if volume.scattering_asymmetry != style.asym {
            volume.scattering_asymmetry = style.asym;
        }
        if volume.absorption != style.absorption {
            volume.absorption = style.absorption;
        }
        if volume.scattering != style.scatter {
            volume.scattering = style.scatter;
        }
    }
    if !seen {
        commands.spawn((
            ShaftFogVolume,
            FogVolume {
                density_factor: density,
                scattering_asymmetry: style.asym,
                absorption: style.absorption,
                scattering: style.scatter,
                light_intensity: intensity,
                ..default()
            },
            Transform::from_translation(cam_pos).with_scale(Vec3::splat(SHAFT_VOLUME_SIDE_M)),
            Name::new("Light_Shaft_Volume"),
        ));
    }
}

/// Adds and removes the volumetric components (see the module docs).
pub fn sync_light_shafts(
    quality: Res<LightQuality>,
    style: Option<Res<ShaftStyle>>,
    env: Option<Res<SessionEnvironment>>,
    mut commands: Commands,
    mut cameras: Query<CameraRayItem<'_>, With<MainCamera>>,
    lights: Query<(Entity, &LightRole, &DirectionalLight, Has<VolumetricLight>)>,
    mut volumes: Query<(Entity, &mut Transform, &mut FogVolume), With<ShaftFogVolume>>,
) {
    let style = style.as_deref().copied().unwrap_or_default();
    let list: Vec<_> = lights.iter().collect();
    let carrier = pick_carrier(
        quality.shafts,
        quality.night_shafts,
        list.iter()
            .map(|(_, role, light, _)| (**role, light.shadow_maps_enabled, light.illuminance)),
    );
    let follow = cameras
        .iter()
        .find(|c| c.4.is_none_or(|cam| cam.is_active))
        .or_else(|| cameras.iter().next())
        .map(|c| (c.1.translation(), c.3));
    let active = carrier.is_some() && follow.is_some();

    for (i, (entity, _, _, has)) in list.iter().enumerate() {
        let want = active && carrier == Some(i);
        if want && !has {
            commands.entity(*entity).insert(VolumetricLight);
        } else if !want && *has {
            commands.entity(*entity).remove::<VolumetricLight>();
        }
    }

    sync_camera_volumetric_fog(&mut cameras, active, &quality, &style, &mut commands);

    let (Some((cam_pos, hdr)), true) = (follow, active) else {
        for (entity, _, _) in &volumes {
            commands.entity(entity).despawn();
        }
        return;
    };
    let role = carrier.map_or(LightRole::Sun, |i| *list[i].1);
    let (density, shafts_att) = env.as_ref().map_or_else(
        || {
            (
                haze_density(crate::weather::FOG_MAX_VISIBILITY_M, &style),
                1.0,
            )
        },
        |e| {
            (
                haze_density(e.conditions().fog_visibility_m, &style),
                e.light_attenuation().shafts,
            )
        },
    );
    let intensity = shaft_intensity(role, shafts_att, hdr, &style);
    sync_fog_volume_entities(
        &mut volumes,
        cam_pos,
        density,
        intensity,
        &style,
        &mut commands,
    );
}

/// Registers [`sync_light_shafts`].
#[derive(Default)]
pub struct SunRaysPlugin;

impl Plugin for SunRaysPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<ShaftStyle>() {
            let style = ShaftStyle::from_env();
            tracing::info!(?style, env = TUNE_ENV, "light shaft style");
            app.insert_resource(style);
        }
        app.init_resource::<LightQuality>().add_systems(
            Update,
            sync_light_shafts
                .in_set(SunRaysSet)
                .after(LightQualitySet)
                .after(crate::EnvironmentSet),
        );
    }
}

#[cfg(test)]
mod tests;
