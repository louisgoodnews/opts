//! Bloom on the main camera, driven by [`LightQuality::bloom`]
//! (`docs/src/architecture/lighting-system.md` section 9, M-0122 deliverable B "Bloom").
//!
//! * [`bloom_for`] is the pure mapping from the effective quality to the camera's [`Bloom`]
//!   component: `None` when bloom is off (the camera carries no bloom at all, so the picture is
//!   byte-identical to a build without this module), otherwise [`restrained_bloom`].
//! * [`restrained_bloom`] is Bevy's natural preset with the strength of the [`BloomLevel`] and a
//!   prefilter threshold of `1.0`, so only pixels brighter than the display white (lamps, the
//!   sun's glints) scatter. Ordinary lit surfaces, such as a white sign, do not get a halo.
//! * [`sync_bloom`] adds, updates or removes the component on [`MainCamera`] only (never on the
//!   mirror, reflection or door cameras). Bloom needs HDR: without [`Hdr`] on the camera it is
//!   removed, because the `Hdr` requirement of [`Bloom`] would otherwise switch HDR back on
//!   against `graphics.hdr = false`.

use bevy::camera::Hdr;
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::prelude::*;

use crate::light_quality::{BloomLevel, LightQuality, LightQualitySet};
use crate::MainCamera;

/// The bloom intensity of a level (`0.0` when off; Bevy's default is `0.15`).
#[must_use]
pub const fn intensity_of(level: BloomLevel) -> f32 {
    match level {
        BloomLevel::Off => 0.0,
        BloomLevel::Low => 0.03,
        BloomLevel::Natural => 0.06,
        BloomLevel::High => 0.10,
    }
}

/// The prefilter threshold in linear HDR (`1.0` is display white).
pub const BLOOM_THRESHOLD: f32 = 1.0;

/// The soft knee of the prefilter threshold.
pub const BLOOM_THRESHOLD_SOFTNESS: f32 = 0.5;

/// The restrained bloom of `level`: Bevy's natural preset, the level's strength, only over-bright
/// pixels.
#[must_use]
pub fn restrained_bloom(level: BloomLevel) -> Bloom {
    Bloom {
        intensity: intensity_of(level),
        prefilter: BloomPrefilter {
            threshold: BLOOM_THRESHOLD,
            threshold_softness: BLOOM_THRESHOLD_SOFTNESS,
        },
        ..Bloom::NATURAL
    }
}

/// The bloom the quality asks for: `None` when [`LightQuality::bloom`] is off.
#[must_use]
pub fn bloom_for(quality: &LightQuality) -> Option<Bloom> {
    quality
        .bloom
        .is_on()
        .then(|| restrained_bloom(quality.bloom))
}

/// The main camera's entity, its current bloom and whether it is HDR.
type MainCameraBloom = (Entity, Option<&'static Bloom>, Has<Hdr>);

/// Keeps [`Bloom`] on the [`MainCamera`] equal to [`bloom_for`], and only while the camera is HDR.
pub fn sync_bloom(
    quality: Res<LightQuality>,
    cameras: Query<MainCameraBloom, With<MainCamera>>,
    mut commands: Commands,
) {
    let want = bloom_for(&quality);
    for (entity, have, hdr) in &cameras {
        match (want.as_ref(), hdr) {
            (Some(wanted), true) => {
                if have.is_none_or(|h| h.intensity != wanted.intensity) {
                    commands.entity(entity).insert(wanted.clone());
                }
            }
            _ => {
                if have.is_some() {
                    commands.entity(entity).remove::<Bloom>();
                }
            }
        }
    }
}

/// Registers [`sync_bloom`] after the quality resolution.
#[derive(Default)]
pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .add_systems(Update, sync_bloom.after(LightQualitySet));
    }
}

#[cfg(test)]
mod tests;
