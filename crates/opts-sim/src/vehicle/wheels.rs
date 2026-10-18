//! The road wheel as a spinning body (M-0097): drive torque and brake torque on one side, the
//! tyre's longitudinal force on the other, the slip between the wheel's rim and the road in
//! between. Under script drive the wheel speeds the scripts read (`Wheel_RotationSpeed_*`,
//! `n_Wheel`) come from here, and no script output can produce a force above the friction limit
//! of the tyre.
//!
//! The tyre force is a friction circle's longitudinal share: linear in the slip up to
//! [`SLIP_PEAK`], then the limit `μ · load`; locked or spinning wheels sit at the limit.

use serde::{Deserialize, Serialize};

use super::axle::WheelState;

/// Mass of a wheel with tyre and brake drum or disc (kg) when nothing says otherwise ❓; its
/// inertia is `m r²`.
pub const WHEEL_MASS_KG: f32 = 90.0;
/// Slip at which the tyre force reaches its limit (rim speed against road speed, as a fraction).
pub const SLIP_PEAK: f32 = 0.12;
/// Road speed below which the slip is taken against this speed (m/s), so a standing wheel does
/// not divide by zero and a held vehicle creeps by less than a millimetre per second.
pub const SLIP_SPEED_FLOOR_M_S: f32 = 0.05;

/// What one wheel did in a step.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct WheelStep {
    /// Longitudinal force of the tyre on the vehicle (N, forward positive).
    pub force_n: f32,
    /// Slip after the step (rim speed minus road speed over the larger of the road speed and the
    /// floor; negative while braking, positive while spinning).
    pub slip: f32,
    /// The tyre is at its friction limit.
    pub saturated: bool,
}

/// Advances one wheel by `dt` seconds.
///
/// * `drive_nm` — torque from the drive (N·m, forward positive);
/// * `brake_nm` — brake torque (N·m, never negative), always against the wheel's rotation;
/// * `load_n` — vertical load on the wheel;
/// * `road_speed_m_s` — the vehicle's speed along the road at this wheel;
/// * `friction` — the friction coefficient `μ`.
///
/// The tyre force is evaluated at the new wheel speed (the linear part implicitly, so a stiff
/// tyre does not make the step unstable), the brake as a Coulomb torque that can stop the wheel
/// and hold it.
pub fn step_wheel(
    w: &mut WheelState,
    drive_nm: f32,
    brake_nm: f32,
    load_n: f32,
    road_speed_m_s: f32,
    friction: f32,
    dt: f32,
) -> WheelStep {
    let r = w.rolling_radius_m.max(0.1);
    let inertia = WHEEL_MASS_KG * r * r;
    let limit = friction.max(0.0) * load_n.max(0.0);
    let vs = road_speed_m_s.abs().max(SLIP_SPEED_FLOOR_M_S);
    let stiffness = limit / SLIP_PEAK; // N per unit of slip
    let omega = w.angular_velocity_rad_s;

    // The wheel speed after the step for a given net torque on the axle: the tyre implicit in
    // its linear range, at the limit beyond the peak slip.
    let solve = |torque: f32| -> (f32, bool) {
        let om = (inertia * omega / dt + torque + r * stiffness * road_speed_m_s / vs)
            / (inertia / dt + r * r * stiffness / vs);
        let slip_free = (om * r - road_speed_m_s) / vs;
        if slip_free.abs() > SLIP_PEAK {
            let force = limit * slip_free.signum();
            (omega + dt / inertia * (torque - force * r), true)
        } else {
            (om, false)
        }
    };
    // The brake acts against the rotation; if it would reverse the wheel it holds it instead.
    let brake = brake_nm.max(0.0);
    let (mut omega_new, mut saturated) = solve(drive_nm);
    let sgn = if omega != 0.0 {
        omega.signum()
    } else {
        omega_new.signum()
    };
    let mut locked = false;
    if brake > 0.0 {
        let (braked, sat) = solve(drive_nm - brake * sgn);
        if braked * sgn <= 0.0 {
            omega_new = 0.0;
            locked = true;
        } else {
            omega_new = braked;
            saturated = sat;
        }
    }

    w.angular_velocity_rad_s = omega_new;
    w.rotation_rad += omega_new * dt;
    let slip = (omega_new * r - road_speed_m_s) / vs;
    let force_n = if locked || saturated {
        (stiffness * slip).clamp(-limit, limit)
    } else {
        // What the torque balance of the step says the tyre delivered.
        let brake_applied = if brake > 0.0 { brake * sgn } else { 0.0 };
        ((drive_nm - brake_applied - inertia * (omega_new - omega) / dt) / r).clamp(-limit, limit)
    };
    saturated |= locked && force_n.abs() >= limit && limit > 0.0;
    WheelStep {
        force_n,
        slip,
        saturated,
    }
}

#[cfg(test)]
mod tests;
