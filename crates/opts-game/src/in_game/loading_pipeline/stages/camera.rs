//! The starting camera of the loaded world: the mode a `--camera` argument or the vehicle asks for,
//! and the free-roam start.

use super::super::*;

/// The camera mode a `--camera` argument asks for, if any.
pub(super) fn requested_camera(args: Option<&AppArgs>) -> Option<String> {
    args.and_then(|a| a.0.camera.as_deref())
        .map(str::to_ascii_lowercase)
}

/// Chooses the starting camera and mode from the loaded vehicle (or an inspection camera).
pub(super) fn setup_camera(
    commands: &mut Commands,
    job: &WorldLoadJob,
    camera_state: &mut CameraState,
    named: Option<&str>,
) {
    if let Some(cam) = job.camera {
        spawn_camera_rig(commands, &cam);
        camera_state.mode = match named {
            Some("cockpit") => {
                camera_state.sub_camera_index = job.camera_std_index;
                CameraMode::Cockpit
            }
            Some("pax") => CameraMode::Passenger,
            Some("ego") => CameraMode::Ego,
            Some("free") => free_roam_start(commands, camera_state, &cam),
            _ => CameraMode::External,
        };
    } else if let Some(tf) = job.inspection_camera() {
        // No vehicle: still give the user a camera to inspect the loaded world.
        tracing::warn!("no vehicle loaded; starting in free-roam over the spawn point");
        spawn_free_camera(commands, tf);
        camera_state.mode = CameraMode::FreeRoam;
        camera_state.free_roam_spawn = tf.translation;
        let (yaw, pitch, _) = tf.rotation.to_euler(EulerRot::YXZ);
        camera_state.free_roam_yaw = yaw;
        camera_state.free_roam_pitch = pitch;
    }
    camera_state.previous_mode = camera_state.mode;
    tracing::info!(mode = ?camera_state.mode, "initial camera mode");
}

/// Free-roam start. Developer aid: `OPTS_DEV_CAM_HEIGHT=<m>` starts high above the spawn,
/// looking down (overview screenshots).
fn free_roam_start(
    commands: &mut Commands,
    camera_state: &mut CameraState,
    cam: &crate::in_game::omsi_world::CameraSetup,
) -> CameraMode {
    let mut start = cam.start;
    if let Some(h) = std::env::var("OPTS_DEV_CAM_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        let focus = cam.start.translation + *cam.start.forward() * h * 0.5;
        start = Transform::from_translation(cam.start.translation + Vec3::Y * h).looking_at(
            Vec3::new(focus.x, cam.start.translation.y, focus.z),
            Vec3::Y,
        );
        commands.queue(move |w: &mut World| {
            let mut q = w.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
            for mut t in q.iter_mut(w) {
                *t = start;
            }
        });
    }
    camera_state.free_roam_spawn = start.translation;
    let (yaw, pitch, _) = start.rotation.to_euler(EulerRot::YXZ);
    camera_state.free_roam_yaw = yaw;
    camera_state.free_roam_pitch = pitch;
    CameraMode::FreeRoam
}
