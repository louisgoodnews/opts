//! Sun and moon god rays with Bevy's `VolumetricFog`
//! (`docs/src/architecture/lighting-system.md` section 6 and 9).
//!
//! One system, [`sync_light_shafts`], makes the volumetric components follow
//! [`LightQuality`](crate::light_quality::LightQuality) and the shadow-carrying light:
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
pub const HAZE_BASE: f32 = 0.002;
/// Factor from the weather's Koschmieder extinction `3.912 / visibility` (per metre) to the
/// volume's `density_factor`. Bevy's extinction is `density_factor × (absorption + scattering)`
/// = `0.6 ×` with the defaults, so 1.0 makes the haze a little thinner than the distance fog.
pub const FOG_TO_SHAFT: f32 = 1.0;
/// Largest `density_factor`: thicker volumes wash the screen out and end every ray early.
pub const HAZE_MAX: f32 = 0.05;
/// `scattering_asymmetry`: strong forward scattering, shafts show when looking towards the light.
pub const SHAFT_ASYMMETRY: f32 = 0.8;
/// Brighter shafts for the moon: its light is about 20 times weaker than the sun's in game units,
/// so with the same intensity its beams would be invisible ❓ (tunable).
pub const MOON_SHAFT_GAIN: f32 = 4.0;
/// Intensity scale without `Hdr` on the camera (the fog adds untonemapped light, colours clip).
pub const LDR_SHAFT_SCALE: f32 = 0.6;
/// `VolumetricFog::jitter` in metres (no effect without temporal anti-aliasing, kept on as in the
/// design).
pub const SHAFT_JITTER: f32 = 0.5;

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
pub fn haze_density(visibility_m: f32) -> f32 {
    let beta = 3.912 / visibility_m.max(1.0);
    (beta * FOG_TO_SHAFT).clamp(HAZE_BASE, HAZE_MAX)
}

/// The `light_intensity` of the volume: the weather's shaft attenuation, the moon's gain and the
/// LDR scale.
#[must_use]
pub fn shaft_intensity(role: LightRole, shafts_attenuation: f32, hdr: bool) -> f32 {
    let gain = match role {
        LightRole::Sun => 1.0,
        LightRole::Moon => MOON_SHAFT_GAIN,
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

/// Adds and removes the volumetric components (see the module docs).
#[allow(clippy::type_complexity)]
pub fn sync_light_shafts(
    quality: Res<LightQuality>,
    env: Option<Res<SessionEnvironment>>,
    mut commands: Commands,
    mut cameras: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&mut VolumetricFog>,
            Has<Hdr>,
            Option<&Camera>,
        ),
        With<MainCamera>,
    >,
    lights: Query<(Entity, &LightRole, &DirectionalLight, Has<VolumetricLight>)>,
    mut volumes: Query<(Entity, &mut Transform, &mut FogVolume), With<ShaftFogVolume>>,
) {
    let list: Vec<_> = lights.iter().collect();
    let carrier = pick_carrier(
        quality.shafts,
        quality.night_shafts,
        list.iter()
            .map(|(_, role, light, _)| (**role, light.shadow_maps_enabled, light.illuminance)),
    );
    // The camera the volume follows: the first active one.
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

    for (entity, _, fog, _, _) in &mut cameras {
        match (active, fog) {
            (true, None) => {
                commands.entity(entity).insert(VolumetricFog {
                    ambient_intensity: 0.0,
                    jitter: SHAFT_JITTER,
                    step_count: quality.shafts.steps(),
                    ..default()
                });
            }
            (true, Some(mut fog)) => {
                if fog.step_count != quality.shafts.steps() {
                    fog.step_count = quality.shafts.steps();
                }
            }
            (false, Some(_)) => {
                commands.entity(entity).remove::<VolumetricFog>();
            }
            (false, None) => {}
        }
    }

    let (Some((cam_pos, hdr)), true) = (follow, active) else {
        for (entity, _, _) in &volumes {
            commands.entity(entity).despawn();
        }
        return;
    };
    let role = carrier.map_or(LightRole::Sun, |i| *list[i].1);
    let (density, shafts_att) = env.as_ref().map_or_else(
        || (haze_density(crate::weather::FOG_MAX_VISIBILITY_M), 1.0),
        |e| {
            (
                haze_density(e.conditions().fog_visibility_m),
                e.light_attenuation().shafts,
            )
        },
    );
    let intensity = shaft_intensity(role, shafts_att, hdr);
    let mut seen = false;
    for (entity, mut t, mut volume) in &mut volumes {
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
    }
    if !seen {
        commands.spawn((
            ShaftFogVolume,
            FogVolume {
                density_factor: density,
                scattering_asymmetry: SHAFT_ASYMMETRY,
                light_intensity: intensity,
                ..default()
            },
            Transform::from_translation(cam_pos).with_scale(Vec3::splat(SHAFT_VOLUME_SIDE_M)),
            Name::new("Light_Shaft_Volume"),
        ));
    }
}

/// Registers [`sync_light_shafts`].
#[derive(Default)]
pub struct SunRaysPlugin;

impl Plugin for SunRaysPlugin {
    fn build(&self, app: &mut App) {
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
