//! The volumetric components of the camera and of the one camera-following fog volume.

use bevy::camera::Hdr;
use bevy::light::{FogVolume, VolumetricFog};
use bevy::prelude::*;

use crate::light_quality::LightQuality;
use crate::MainCamera;
use super::*;

pub(super) type CameraRayItem<'a> = (
    Entity,
    &'a GlobalTransform,
    Option<&'a mut VolumetricFog>,
    Has<Hdr>,
    Option<&'a Camera>,
);

pub(super) fn sync_camera_volumetric_fog(
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

pub(super) fn sync_fog_volume_entities(
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
