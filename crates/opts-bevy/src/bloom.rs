//! Bloom on the main camera, driven by [`LightQuality::bloom`]
//! (`docs/src/architecture/lighting-system.md` section 9, M-0122 deliverable B "Bloom").
//!
//! * [`bloom_for`] is the pure mapping from the effective quality to the camera's [`Bloom`]
//!   component: `None` when bloom is off (the camera carries no bloom at all, so the picture is
//!   byte-identical to a build without this module), otherwise [`restrained_bloom`].
//! * [`restrained_bloom`] is Bevy's natural preset with a low intensity and a prefilter threshold
//!   of `1.0`, so only pixels brighter than the display white (lamps, the sun's glints) scatter.
//!   Ordinary lit surfaces, such as a white sign, do not get a halo.
//! * [`sync_bloom`] adds or removes the component on [`MainCamera`] only (never on the mirror,
//!   reflection or door cameras). Bloom needs HDR: without [`Hdr`] on the camera it is removed,
//!   because the `Hdr` requirement of [`Bloom`] would otherwise switch HDR back on against
//!   `graphics.hdr = false`.

use bevy::camera::Hdr;
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::prelude::*;

use crate::light_quality::{LightQuality, LightQualitySet};
use crate::MainCamera;

/// The bloom intensity (Bevy's default is `0.15`).
pub const BLOOM_INTENSITY: f32 = 0.06;

/// The prefilter threshold in linear HDR (`1.0` is display white).
pub const BLOOM_THRESHOLD: f32 = 1.0;

/// The soft knee of the prefilter threshold.
pub const BLOOM_THRESHOLD_SOFTNESS: f32 = 0.5;

/// The restrained bloom: Bevy's natural preset, lower intensity, only over-bright pixels.
#[must_use]
pub fn restrained_bloom() -> Bloom {
    Bloom {
        intensity: BLOOM_INTENSITY,
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
    quality.bloom.then(restrained_bloom)
}

/// Keeps [`Bloom`] on the [`MainCamera`] equal to [`bloom_for`], and only while the camera is HDR.
pub fn sync_bloom(
    quality: Res<LightQuality>,
    cameras: Query<(Entity, Has<Bloom>, Has<Hdr>), With<MainCamera>>,
    mut commands: Commands,
) {
    let want = bloom_for(&quality).is_some();
    for (entity, has, hdr) in &cameras {
        let wanted_here = want && hdr;
        if wanted_here && !has {
            commands.entity(entity).insert(restrained_bloom());
        } else if !wanted_here && has {
            commands.entity(entity).remove::<Bloom>();
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
