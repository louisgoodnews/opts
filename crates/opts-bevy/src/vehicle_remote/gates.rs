//! The input gates of the remote controls and the odometer of the drive.

use super::*;

/// The negation of the run conditions of the vehicle input system (no vehicle input now).
#[must_use]
pub fn vehicle_input_closed(
    camera: Option<Res<crate::camera::CameraState>>,
    modal: Option<Res<crate::reporting::state::ModalActive>>,
    menu: Option<Res<crate::reporting::state::MenuModalActive>>,
    viewpoint: Option<Res<crate::camera::ViewpointState>>,
) -> bool {
    let accepts = camera.is_none_or(|s| s.mode != crate::camera::CameraMode::FreeRoam)
        && !viewpoint.is_some_and(|v| v.editing);
    let open = modal.is_some_and(|m| m.0) || menu.is_some_and(|m| m.0);
    !accepts || open
}

/// Drops held controls while the input gates are closed, so nothing fires when they reopen.
pub fn flush_remote_when_gated(mut remote: ResMut<RemoteControls>) {
    if remote.is_active() {
        remote.release_all();
    }
}

/// Counts the distance driven; a jump of more than 50 m in one frame is a placement, not driving.
pub fn odometer_system(
    mut remote: ResMut<RemoteControls>,
    added: Query<(), Added<PlayerVehicle>>,
    vehicles: Query<&VehicleController, With<PlayerVehicle>>,
) {
    let Ok(c) = vehicles.single() else {
        return;
    };
    let ch = &c.sim.chassis;
    if !added.is_empty() || remote.start_pose.is_none() {
        remote.start_pose = Some((ch.position, ch.heading_rad, ch.elevation_z));
        remote.restart_odometer();
        if let Some((metres, _)) = remote.pending_odometer.take() {
            remote.odometer_m = metres;
        }
    }
    if let Some((_, frames)) = &mut remote.pending_odometer {
        *frames = frames.saturating_sub(1);
        if *frames == 0 {
            remote.pending_odometer = None;
        }
    }
    if let Some(last) = remote.last_position {
        let d = ch.position.distance(last);
        if d < 50.0 {
            remote.odometer_m += f64::from(d);
        }
    }
    remote.last_position = Some(ch.position);
}
