//! Writing the joint variables and wheel rotations of the chain into the vehicle's scripts.

use crate::vehicle_script::VehicleScripts;

use super::segment::SegmentWheel;

pub(super) fn apply_articulation_scripts(
    scripts: &VehicleScripts,
    joints: &[(usize, f32, f32)],
    alphas: &[f64],
    wheel_vars: &[(usize, f32, Vec<SegmentWheel>)],
    rest: f32,
) {
    if let Ok(mut engine) = scripts.engine.lock() {
        for (k, beta, gamma) in joints {
            engine.set(&format!("articulation_{k}_beta"), *beta);
            engine.set(&format!("articulation_{k}_gamma"), *gamma);
        }
        for (k, a) in alphas.iter().enumerate() {
            engine.set(&format!("articulation_{k}_alpha"), *a as f32);
        }
        for (first, speed, wheels) in wheel_vars {
            for (j, w) in wheels.iter().enumerate() {
                let i = first + j;
                let rpm = speed / w.radius * 60.0 / std::f32::consts::TAU;
                for side in ["L", "R"] {
                    engine.set(&format!("wheel_rotation_{i}_{side}"), w.rotation_rad);
                    engine.set(&format!("axle_steering_{i}_{side}"), 0.0);
                    engine.set(&format!("axle_suspension_{i}_{side}"), rest);
                }
                engine.set(&format!("wheel_rotationspeed_{i}_l"), rpm);
                engine.set(&format!("wheel_rotationspeed_{i}_r"), rpm);
            }
        }
    }
}
