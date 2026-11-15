//! The depth prepass of the main camera: its gate while the cones need it, and adding and removing it.

use super::*;

/// The prepass gate (`0..=1`): 1 without the depth fade; with it, 0 until the camera has had the
/// prepass for [`PREPASS_READY_S`] and [`PREPASS_READY_FRAMES`], then rising over
/// [`CONE_FADE_IN_S`].
#[must_use]
pub fn prepass_gate(depth_fade: bool, age_s: f32, frames: u32) -> f32 {
    if !depth_fade {
        return 1.0;
    }
    if frames < PREPASS_READY_FRAMES || age_s < PREPASS_READY_S {
        return 0.0;
    }
    ((age_s - PREPASS_READY_S) / CONE_FADE_IN_S).clamp(0.0, 1.0)
}

pub(super) fn update_prepass_gate(
    pool: &mut ConePool,
    quality: &LightQuality,
    camera: Option<(Vec3, Vec3, bool)>,
    dt: f32,
) -> f32 {
    if quality.cone_depth_fade && camera.is_some_and(|c| c.2) {
        pool.prepass_age_s += dt;
        pool.prepass_frames += 1;
    } else {
        pool.prepass_age_s = 0.0;
        pool.prepass_frames = 0;
    }
    let gate = prepass_gate(
        quality.cone_depth_fade,
        pool.prepass_age_s,
        pool.prepass_frames,
    );
    pool.gate = gate;
    gate
}

/// Marks a [`DepthPrepass`] that [`cone_depth_prepass_system`] added (so only that one is
/// removed again).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ConeDepthPrepass;

/// Whether the main camera should have the depth prepass: the depth fade is on, cones are
/// allowed at all, and a cone shines (or shone less than [`PREPASS_RELEASE_S`] ago, while the
/// camera already has it).
#[must_use]
pub fn wants_depth_prepass(
    quality: &LightQuality,
    active_cones: usize,
    idle_s: f32,
    has_prepass: bool,
) -> bool {
    let allowed = quality.cone_depth_fade
        && quality.cone_cap > 0
        && (quality.lantern_cones || quality.headlight_cones);
    allowed && (active_cones > 0 || (has_prepass && idle_s < PREPASS_RELEASE_S))
}

/// Adds the depth prepass to the main camera while the cones need it and removes it again.
pub fn cone_depth_prepass_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    quality: Res<LightQuality>,
    stats: Res<ConeStats>,
    mut pool: ResMut<ConePool>,
    cameras: Query<(Entity, Has<DepthPrepass>, Has<ConeDepthPrepass>), With<crate::MainCamera>>,
) {
    if stats.active > 0 {
        pool.idle_s = 0.0;
    } else {
        pool.idle_s += time.delta_secs();
    }
    for (camera, has, ours) in &cameras {
        let want = wants_depth_prepass(&quality, stats.active, pool.idle_s, has);
        if want && !has {
            commands
                .entity(camera)
                .insert((DepthPrepass, ConeDepthPrepass));
        } else if !want && has && ours {
            commands
                .entity(camera)
                .remove::<(DepthPrepass, ConeDepthPrepass)>();
        }
    }
}
