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

use bevy::light::{FogVolume, VolumetricLight};
use bevy::prelude::*;

use crate::environment::SessionEnvironment;
use crate::light_quality::{LightQuality, LightQualitySet};
use crate::MainCamera;

mod shaping;
mod style;
mod sync;

pub use shaping::*;
pub use style::*;
use sync::*;

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
