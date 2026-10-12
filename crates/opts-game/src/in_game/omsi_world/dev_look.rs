//! Developer aid for visual checks of scenery objects: `OPTS_DEV_LOOK_AT=<.sco substring>` puts
//! the free camera in front of the first placed object whose `.sco` path contains the substring
//! (case-insensitive), e.g. a signal head for the traffic-light screenshots of
//! `docs/src/architecture/scenery-runtime.md`. Use with `--camera free`.

use bevy::prelude::*;
use opts_bevy::{CameraMode, CameraState};

/// The object found for `OPTS_DEV_LOOK_AT` and whether the camera was placed.
#[derive(Debug, Clone, Default)]
pub struct DevLookTarget {
    /// Lower-case substring from the environment (`None`: aid off).
    pub pattern: Option<String>,
    /// Root entity of the first matching object.
    pub entity: Option<Entity>,
    /// Frames the camera has been placed after loading finished (it is held for a while
    /// because the loading pipeline sets the camera once more when it completes).
    pub placed_frames: u32,
}

impl DevLookTarget {
    /// Reads `OPTS_DEV_LOOK_AT`.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            pattern: std::env::var("OPTS_DEV_LOOK_AT")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_lowercase()),
            ..Self::default()
        }
    }

    /// Remembers `root` when `sco_file` matches and nothing was chosen yet.
    pub fn offer(&mut self, sco_file: &str, root: Entity) {
        if self.entity.is_none()
            && self
                .pattern
                .as_deref()
                .is_some_and(|p| sco_file.to_lowercase().contains(p))
        {
            tracing::info!(sco = sco_file, "OPTS_DEV_LOOK_AT: target object chosen");
            self.entity = Some(root);
        }
    }
}

/// Frames the camera is held at the target after loading.
const HOLD_FRAMES: u32 = 30;

/// Places the free camera 12 m in front of the chosen object (OMSI +y side), looking at it.
pub fn place_dev_camera(
    job: Option<ResMut<super::WorldLoadJob>>,
    load: Res<crate::in_game::types::SimulationLoadState>,
    mut state: ResMut<CameraState>,
    objects: Query<&GlobalTransform, Without<Camera3d>>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let Some(mut job) = job else { return };
    let target = &mut job.scenery.dev_look;
    let Some(entity) = target.entity else {
        return;
    };
    if !load.is_complete || target.placed_frames >= HOLD_FRAMES {
        return;
    }
    if state.mode != CameraMode::FreeRoam {
        return;
    }
    let Ok(object) = objects.get(entity) else {
        return;
    };
    let origin = object.translation();
    if origin == Vec3::ZERO {
        return; // not placed yet (transforms propagate next frame)
    }
    let front = object.rotation() * Vec3::NEG_Z;
    let focus = origin + Vec3::Y * 2.5;
    let eye = origin + front * 12.0 + Vec3::Y * 2.0;
    let pose = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
    for mut t in &mut cameras {
        *t = pose;
    }
    let (yaw, pitch, _) = pose.rotation.to_euler(EulerRot::YXZ);
    state.free_roam_yaw = yaw;
    state.free_roam_pitch = pitch;
    if target.placed_frames == 0 {
        tracing::info!(?eye, ?focus, "OPTS_DEV_LOOK_AT: camera placed");
    }
    target.placed_frames += 1;
}
