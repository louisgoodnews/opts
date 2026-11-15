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
        let ch = &controller.sim.chassis;
        controller.render_prev = Some((ch.position, ch.heading_rad, ch.elevation_z));
        controller.sim.step(dt);
    }
}

/// System synchronizing chassis simulation coordinates to Bevy [`Transform`] components.
pub fn vehicle_transform_sync_system(
    fixed: Res<Time<Fixed>>,
    mut query: Query<(&VehicleController, &mut Transform)>,
) {
    let alpha = fixed.overstep_fraction().clamp(0.0, 1.0);
    for (controller, mut transform) in &mut query {
        let ch = &controller.sim.chassis;
        // Between two fixed steps the pose is interpolated from the previous to the current one
        // (the heights come from the ground follower at the frame rate and are not blended).
        let (sim_pos, heading_rad) = match controller.render_prev {
            Some((p0, h0, _)) => {
                let dh = (ch.heading_rad - h0 + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                (p0.lerp(ch.position, alpha), h0 + dh * alpha)
            }
            None => (ch.position, ch.heading_rad),
        };
        let sim_elevation = ch.elevation_z;

        // Convert OMSI simulation position (Z-up) to Bevy space (Y-up)
        let bevy_pos = omsi_to_bevy_pos(Vec3::new(sim_pos.x, sim_pos.y, sim_elevation));
        transform.translation = bevy_pos;

        // Yaw heading: In OMSI, +yaw turns left around Z; in Bevy, -rotation turns right around +Y
        transform.rotation = Quat::from_rotation_y(-heading_rad);
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
