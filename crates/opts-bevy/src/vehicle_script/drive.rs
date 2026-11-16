//! Script-driven drivetrain (`M_Wheel` and brake forces) and vehicle start-up systems.

use super::*;

/// Seconds of throttle with an all-zero `M_Wheel` after which a vehicle falls back to the
/// built-in drivetrain (its const file's torque curve).
const NO_TORQUE_FALLBACK_S: f32 = 4.0;

mod feed;
mod startup;

pub use feed::script_drive_system;
pub use startup::script_startup_system;

/// What the scripts hand to the physics: the torque at the driven wheels (`M_Wheel`, N·m) and the
/// total brake force (N). `None` when no script writes `M_Wheel` (the vehicle is not
/// script-driven). Player vehicles write the force of every wheel (`Axle_Brakeforce_<n>_<l|r>`);
/// the total `Brakeforce` belongs to the AI scripts (`{frame_ai}`), so the wheel forces win.
#[must_use]
pub fn script_outputs(engine: &ScriptEngine) -> Option<opts_sim::vehicle::ScriptDrive> {
    if !engine.writes("m_wheel") {
        return None;
    }
    let wheel_names: Vec<String> = (0..8)
        .flat_map(|i| {
            [
                format!("axle_brakeforce_{i}_l"),
                format!("axle_brakeforce_{i}_r"),
            ]
        })
        .collect();
    // A wheel's brake cannot push: the Valiant's script gives a released brake a negative force
    // (cylinder at ambient minus the response pressure), which must not cancel another wheel's.
    let brake = if wheel_names.iter().any(|n| engine.writes(n)) {
        wheel_names.iter().map(|n| engine.get(n).max(0.0)).sum()
    } else {
        engine.get("brakeforce")
    };
    Some(opts_sim::vehicle::ScriptDrive {
        wheel_torque_nm: engine.get("m_wheel") * opts_sim::vehicle::M_WHEEL_UNIT,
        brake_force_n: brake.max(0.0) * opts_sim::vehicle::BRAKE_FORCE_UNIT,
    })
}

/// What [`script_drive_system`] remembers of one vehicle's scripts.
#[derive(Debug, Default)]
pub struct DriveWatch {
    idle_s: f32,
    fallback: bool,
    warned: bool,
}
