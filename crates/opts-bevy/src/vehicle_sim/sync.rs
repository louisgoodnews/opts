//! Fixed-step physics advance, chassis-to-transform projection and wheel animation.

use bevy::prelude::*;

use super::controller::{VehicleController, WheelSide, WheelVisual};
use crate::coords::omsi_to_bevy_pos;

/// Fixed-update system executing deterministic vehicle physics simulation ticks.
pub fn vehicle_physics_step_system(time: Res<Time>, mut query: Query<&mut VehicleController>) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for mut controller in &mut query {
        controller.render_prev = Some(controller.pose());
        controller.sim.step(dt);
    }
}

/// System synchronizing the rendered pose of each vehicle to its Bevy [`Transform`]: the position
/// and the yaw, interpolated between the last two fixed steps. The ground follower adds the
/// suspension and the pitch and roll on top of it, from the same [`VehicleController::render_pose`].
pub fn vehicle_transform_sync_system(
    fixed: Res<Time<Fixed>>,
    mut query: Query<(&VehicleController, &mut Transform)>,
) {
    let alpha = fixed.overstep_fraction().clamp(0.0, 1.0);
    for (controller, mut transform) in &mut query {
        let pose = controller.render_pose(alpha);
        // The heights come from the ground follower at the frame rate; the chassis elevation is
        // the simulation's own.
        let sim_elevation = controller.sim.chassis.elevation_z;

        // Convert OMSI simulation position (Z-up) to Bevy space (Y-up)
        transform.translation =
            omsi_to_bevy_pos(Vec3::new(pose.position.x, pose.position.y, sim_elevation));

        // Yaw heading: In OMSI, +yaw turns left around Z; in Bevy, -rotation turns right around +Y
        transform.rotation = Quat::from_rotation_y(-pose.heading_rad);
    }
}

/// System animating child wheel entity transforms according to axle steering and spin rotation.
pub fn wheel_transform_sync_system(
    vehicles: Query<(&VehicleController, &Children)>,
    mut wheels: Query<(&WheelVisual, &mut Transform)>,
) {
    for (controller, children) in &vehicles {
        for &child_entity in children {
            if let Ok((visual, mut transform)) = wheels.get_mut(child_entity) {
                if let Some(axle) = controller.sim.axles.get(visual.axle_index) {
                    let wheel = match visual.side {
                        WheelSide::Left => &axle.left_wheel,
                        WheelSide::Right => &axle.right_wheel,
                    };

                    let steer_rot = Quat::from_rotation_y(-wheel.steer_angle_rad);
                    let roll_rot = Quat::from_rotation_x(-wheel.rotation_rad);

                    transform.rotation = steer_rot * roll_rot;
                    transform.translation = visual.rest_translation;
                }
            }
        }
    }
}
