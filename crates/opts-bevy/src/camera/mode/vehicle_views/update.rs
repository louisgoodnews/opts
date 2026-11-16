//! Vehicle view updates: the cockpit, passenger and external camera transforms each frame.

use super::*;

/// The player's vehicle, which is not the camera.
/// A coupled unit's pose, and whether it hangs under a parent.
type UnitFrame = (&'static Transform, &'static GlobalTransform, Has<ChildOf>);
/// Units are neither the player's root nor the camera.
type UnitsOnly = (Without<PlayerVehicle>, Without<crate::MainCamera>);
type PlayerNotCamera = (With<PlayerVehicle>, Without<crate::MainCamera>);

/// Key 2: the active `[add_camera_driver]` preset, composed with the vehicle's world transform.
pub fn update_cockpit_camera(
    state: Res<CameraState>,
    vehicle: Query<(&Transform, &BusVehicle), PlayerNotCamera>,
    units: Query<UnitFrame, UnitsOnly>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut viewpoint: ResMut<ViewpointState>,
    mut camera: Query<(&mut Transform, &mut Projection), With<crate::MainCamera>>,
) {
    if state.mode != CameraMode::Cockpit {
        return;
    }
    let (Ok((vehicle_tf, bus)), Ok((mut tf, mut proj))) = (vehicle.single(), camera.single_mut())
    else {
        return;
    };
    let Some((unit, cam)) = bus.driver_camera(state.sub_camera_index) else {
        return;
    };
    // The vehicle's own (root) transform of this frame, not its `GlobalTransform`: this system
    // runs before the propagation (see `OptsCameraPlugin`), when the global one still holds the
    // last frame's pose. The camera then moves with the vehicle in the same frame.
    // The cab of a coupled unit (a tram's second head) is in that unit's frame; units are placed
    // in the world frame by the rail systems, so their own `Transform` is this frame's pose.
    let gt = match unit.and_then(|e| units.get(e).ok()) {
        Some((t, _, false)) => GlobalTransform::from(*t),
        Some((_, g, true)) => *g,
        None => GlobalTransform::from(*vehicle_tf),
    };
    *tf = preset_world_transform(&gt, cam, state.view_yaw, state.view_pitch);
    // Eye position and direction are the vehicle's own `[add_camera_driver]` preset (the player
    // character is hidden in this view, see `crate::human::hide_driver_in_seat_views`).
    // The viewpoint editor's offset (right, up, forward in the driver's frame) and FOV.
    if (viewpoint.default_fov_deg - cam.fov_deg).abs() > 1e-4 {
        viewpoint.default_fov_deg = cam.fov_deg;
    }
    let (_, vrot, _) = gt.to_scale_rotation_translation();
    tf.translation += vrot * viewpoint.active.local_offset();
    set_fov(
        &mut proj,
        fov_y_from_horizontal(viewpoint.effective_fov_deg(), window_aspect(&windows)),
    );
}

/// Key 4: the active `[add_camera_pax]` preset, in the frame of the segment (tractor or trailer)
/// that defines it.
pub fn update_passenger_camera(
    state: Res<CameraState>,
    vehicle: Query<(&GlobalTransform, &BusVehicle), With<PlayerVehicle>>,
    segments: Query<&GlobalTransform, Without<PlayerVehicle>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<crate::MainCamera>>,
) {
    if state.mode != CameraMode::Passenger {
        return;
    }
    let (Ok((gt, bus)), Ok((mut tf, mut proj))) = (vehicle.single(), camera.single_mut()) else {
        return;
    };
    let Some((segment, cam)) = bus.pax_camera(state.sub_camera_index) else {
        return;
    };
    let frame = segment.and_then(|e| segments.get(e).ok()).unwrap_or(gt);
    *tf = preset_world_transform(frame, cam, state.view_yaw, state.view_pitch);
    set_fov(
        &mut proj,
        fov_y_from_horizontal(cam.fov_deg, window_aspect(&windows)),
    );
}

/// Key 3: orbit around the vehicle's `[set_camera_outside_center]` (or its origin), plus pan offset.
pub fn update_external_camera(
    state: Res<CameraState>,
    vehicle: Query<(&GlobalTransform, &BusVehicle), With<PlayerVehicle>>,
    segments: Query<&GlobalTransform, (With<ArticulatedSegment>, Without<crate::MainCamera>)>,
    mut camera: Query<(&mut Transform, &mut Projection), With<crate::MainCamera>>,
) {
    if state.mode != CameraMode::External {
        return;
    }
    let (Ok((gt, bus)), Ok((mut tf, mut proj))) = (vehicle.single(), camera.single_mut()) else {
        return;
    };
    let (_, vrot, vpos) = gt.to_scale_rotation_translation();
    let local_centre = bus
        .outside_camera_center
        .map_or(Vec3::ZERO, omsi_to_bevy_pos);
    let centre = vpos + vrot * local_centre + state.pan_offset;
    // Orbit angles are relative to the vehicle's heading so the view turns with it.
    let orbit =
        vrot * Quat::from_euler(EulerRot::YXZ, state.external_yaw, state.external_pitch, 0.0);
    // The camera sits behind the centre looking forward: +Z (back) rotated by the orbit.
    // Articulated chains reach far behind the head unit: stay outside of every segment.
    let reach = segments
        .iter()
        .map(|g| g.translation().distance(centre))
        .filter(|d| *d < 25.0)
        .fold(0.0_f32, f32::max);
    let distance = state
        .external_zoom
        .max(if reach > 0.0 { reach + 6.0 } else { 0.0 });
    tf.translation = centre + orbit * Vec3::Z * distance;
    tf.look_at(centre, Vec3::Y);
    set_fov(&mut proj, DEFAULT_FOV_Y);
}
