//! The `camera` section: the camera mode, sub-camera, free camera pose, orbit and look offsets.

use super::*;

// ---- camera ----------------------------------------------------------------------------------

/// Data of the `camera` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraData {
    /// Mode as its number key (1 ego, 2 cockpit, 3 external, 4 passenger, 5 free).
    pub mode: u8,
    /// The mode before it (key 0).
    pub previous_mode: u8,
    /// Index into the sub-camera list of the mode.
    pub sub_camera: usize,
    /// Free camera speed (m/s).
    pub free_roam_speed: f32,
    /// Free camera yaw (rad).
    pub free_roam_yaw: f32,
    /// Free camera pitch (rad).
    pub free_roam_pitch: f32,
    /// Absolute OMSI position of the free camera `[x, y, z]` (m).
    pub free_roam_position: [f64; 3],
    /// External view distance (m).
    pub external_zoom: f32,
    /// External view yaw (rad).
    pub external_yaw: f32,
    /// External view pitch (rad).
    pub external_pitch: f32,
    /// External view orbit centre offset (m).
    pub pan_offset: [f32; 3],
    /// Look offsets in cockpit and passenger views (rad): yaw, pitch.
    pub view: [f32; 2],
    /// Look offsets in the ego view (rad): yaw, pitch.
    pub ego: [f32; 2],
}

pub(super) fn mode_of(key: u8) -> Option<CameraMode> {
    CameraMode::ALL.into_iter().find(|m| m.key_number() == key)
}

pub(super) fn camera_capture(world: &mut World) -> Result<Option<Value>, String> {
    let Some(state) = world.get_resource::<CameraState>().cloned() else {
        return Ok(None);
    };
    let translation = world
        .query_filtered::<&Transform, With<opts_bevy::MainCamera>>()
        .iter(world)
        .next()
        .map_or(Vec3::ZERO, |t| t.translation);
    let omsi = opts_bevy::bevy_to_omsi_pos(translation);
    let (x, y) = ops::local_to_abs(world, Vec2::new(omsi.x, omsi.y));
    to_value(&CameraData {
        mode: state.mode.key_number(),
        previous_mode: state.previous_mode.key_number(),
        sub_camera: state.sub_camera_index,
        free_roam_speed: state.free_roam_speed,
        free_roam_yaw: state.free_roam_yaw,
        free_roam_pitch: state.free_roam_pitch,
        free_roam_position: [x, y, f64::from(omsi.z)],
        external_zoom: state.external_zoom,
        external_yaw: state.external_yaw,
        external_pitch: state.external_pitch,
        pan_offset: state.pan_offset.to_array(),
        view: [state.view_yaw, state.view_pitch],
        ego: [state.ego_yaw, state.ego_pitch],
    })
}

pub(super) fn camera_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let d: CameraData = parse(data, "camera")?;
    let wanted = mode_of(d.mode).unwrap_or(CameraMode::External);
    let allowed = world
        .get_resource::<CameraAvailability>()
        .is_none_or(|a| a.allows(wanted));
    let local = ops::abs_to_local(world, d.free_roam_position[0], d.free_roam_position[1]);
    let position =
        opts_bevy::omsi_to_bevy_pos(Vec3::new(local.x, local.y, d.free_roam_position[2] as f32));
    {
        let mut state = world.resource_mut::<CameraState>();
        // A mode the restored session cannot offer (the vehicle failed to come back) falls back
        // to the free camera at its saved place.
        state.mode = if allowed {
            wanted
        } else {
            CameraMode::FreeRoam
        };
        state.previous_mode = mode_of(d.previous_mode).unwrap_or(state.mode);
        state.sub_camera_index = d.sub_camera;
        state.free_roam_speed = d.free_roam_speed.max(opts_bevy::FREE_ROAM_MIN_SPEED);
        state.free_roam_yaw = d.free_roam_yaw;
        state.free_roam_pitch = d.free_roam_pitch;
        state.external_zoom = d.external_zoom;
        state.external_yaw = d.external_yaw;
        state.external_pitch = d.external_pitch;
        state.pan_offset = Vec3::from_array(d.pan_offset);
        state.view_yaw = d.view[0];
        state.view_pitch = d.view[1];
        state.ego_yaw = d.ego[0];
        state.ego_pitch = d.ego[1];
        if state.mode == CameraMode::FreeRoam {
            state.free_roam_spawn = position;
            state.free_roam_spawn_yaw = d.free_roam_yaw;
            state.free_roam_spawn_pitch = d.free_roam_pitch;
        }
    }
    if world.resource::<CameraState>().mode == CameraMode::FreeRoam {
        let rotation = Quat::from_euler(EulerRot::YXZ, d.free_roam_yaw, d.free_roam_pitch, 0.0);
        let mut cameras = world.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
        for mut t in cameras.iter_mut(world) {
            t.translation = position;
            t.rotation = rotation;
        }
    }
    Ok(())
}
