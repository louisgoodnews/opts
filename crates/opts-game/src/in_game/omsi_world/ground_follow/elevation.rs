//! The vertical motion of the player's vehicle: elevation, tilt and the falling body.

use super::*;

/// The player's vertical motion between frames.
#[derive(Default, Clone, Copy)]
pub struct BodyMotion {
    /// Falling speed (m/s, positive down).
    fall: f32,
    pub(super) pitch: f32,
    pub(super) roll: f32,
    /// Vertical speed of the elevation in the last frame (m/s, up positive).
    rise: f32,
}

/// Moves the vehicle's elevation towards `target`: a jump (respawn, tile change) at once, a drop
/// by gravity, a rise smoothly, and records the vertical acceleration of following the ground
/// (kerbs, slopes; M-0146).
pub(super) fn settle_elevation(
    ch: &mut opts_sim::vehicle::chassis::ChassisState,
    motion: &mut BodyMotion,
    (target, z): (f32, f32),
    dt: f32,
) {
    let teleport = (target - z).abs() > 12.0;
    if teleport {
        // Teleports (respawn, tile change): no motion between the two heights.
        ch.elevation_z = target;
        motion.fall = 0.0;
    } else if target < z - 0.02 {
        motion.fall += GRAVITY * dt;
        ch.elevation_z = (z - motion.fall * dt).max(target);
        if ch.elevation_z <= target {
            motion.fall = 0.0;
        }
    } else {
        motion.fall = 0.0;
        ch.elevation_z = z + (target - z) * (1.0 - (-RISE_RATE * dt).exp());
    }
    // The vertical acceleration of following the ground (kerbs, slopes; M-0146).
    let rise = if teleport || dt <= 0.0 {
        0.0
    } else {
        (ch.elevation_z - z) / dt
    };
    ch.ground_vertical_acc_m_s2 = if teleport || dt <= 0.0 {
        0.0
    } else {
        ((rise - motion.rise) / dt).clamp(-MAX_GROUND_ACC, MAX_GROUND_ACC)
    };
    motion.rise = rise;
}

/// Eases the pitch and roll of the plane through the wheels towards what the ground asks.
pub(super) fn blend_tilt(
    motion: &mut BodyMotion,
    heights: [f32; 4],
    (wheelbase, track): (f32, f32),
    dt: f32,
) {
    let [fl, fr, rl, rr] = heights;
    let wheelbase = wheelbase.max(1.0);
    let pitch_goal = (((fl + fr) - (rl + rr)) * 0.5 / wheelbase)
        .atan()
        .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
    let roll_goal = (((fr + rr) - (fl + rl)) * 0.5 / track)
        .atan()
        .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
    let blend = 1.0 - (-TILT_RATE * dt).exp();
    motion.pitch += (pitch_goal - motion.pitch) * blend;
    motion.roll += (roll_goal - motion.roll) * blend;
}
