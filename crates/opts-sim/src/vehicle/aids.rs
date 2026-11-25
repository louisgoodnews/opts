//! The ABS and ASR driver aids (ISS-043, M-0165): anti-lock braking and traction control of the
//! wheels the scripts drive.
//!
//! Both work on the per-wheel slip. Under script drive the wheel speeds come from the wheel model
//! (`wheels.rs`), so a locking or spinning wheel shows in its slip; without script drive the wheels
//! turn with the chassis and both aids see no slip. ABS releases the brake torque of a locking
//! wheel; ASR cuts the drive torque of all driven wheels while the fastest one spins.

use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use super::axle::WheelState;
use super::wheels::SLIP_SPEED_FLOOR_M_S;

/// ABS: slip (rim speed against road speed, a fraction, negative while braking) at which a wheel
/// counts as locking. TODO(owner)
pub const ABS_SLIP_LIMIT: f32 = 0.15;
/// ABS: share of a wheel's brake torque taken off per step while it locks. TODO(owner)
pub const ABS_RELEASE_STEP: f32 = 0.25;
/// ABS: share of the brake torque given back per step once the wheel turns again. TODO(owner)
pub const ABS_REAPPLY_STEP: f32 = 0.02;
/// ABS: road speed below which the aid is off (m/s). TODO(owner)
pub const ABS_MIN_SPEED_M_S: f32 = 2.0;
/// ASR: slip of the fastest driven wheel above which the drive torque is cut. TODO(owner)
pub const ASR_SLIP_LIMIT: f32 = 0.10;
/// ASR: share of the drive torque taken off per step while a driven wheel spins. TODO(owner)
pub const ASR_CUT_STEP: f32 = 0.05;
/// ASR: share of the drive torque given back per step. TODO(owner)
pub const ASR_REAPPLY_STEP: f32 = 0.01;
/// ASR: lowest share of the drive torque the aid leaves. TODO(owner)
pub const ASR_MIN_SHARE: f32 = 0.3;

/// The switches of the two aids and the shares they let through (1 is no intervention).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverAids {
    /// ABS on (the setting `gameplay.abs`).
    pub abs: bool,
    /// ASR on (the setting `gameplay.asr`).
    pub asr: bool,
    /// Share of each wheel's brake torque the ABS lets through; wheel `2n` is the left wheel of
    /// axle `n`, `2n + 1` the right one. Grows on demand.
    #[serde(default)]
    pub brake_share: Vec<f32>,
    /// Share of the drive torque the ASR lets through.
    #[serde(default = "full_share")]
    pub drive_share: f32,
}

fn full_share() -> f32 {
    1.0
}

impl Default for DriverAids {
    fn default() -> Self {
        Self {
            abs: false,
            asr: false,
            brake_share: Vec::new(),
            drive_share: 1.0,
        }
    }
}

/// Slip of a wheel as the wheel model defines it: rim speed minus road speed over the larger of
/// the road speed and [`SLIP_SPEED_FLOOR_M_S`].
#[must_use]
pub fn wheel_slip(w: &WheelState, road_speed_m_s: f32) -> f32 {
    let r = w.rolling_radius_m.max(0.1);
    let vs = road_speed_m_s.abs().max(SLIP_SPEED_FLOOR_M_S);
    (w.angular_velocity_rad_s * r - road_speed_m_s) / vs
}

impl DriverAids {
    /// The brake share of wheel `index` for the next step. A locking wheel (slip below
    /// `-ABS_SLIP_LIMIT` at a road speed of at least `ABS_MIN_SPEED_M_S`) loses
    /// `ABS_RELEASE_STEP`, any other wheel regains `ABS_REAPPLY_STEP` up to full.
    pub fn brake_share_for(&mut self, index: usize, slip: f32, road_speed_m_s: f32) -> f32 {
        if !self.abs {
            return 1.0;
        }
        if self.brake_share.len() <= index {
            self.brake_share.resize(index + 1, 1.0);
        }
        let before = self.brake_share[index];
        let locking = road_speed_m_s.abs() >= ABS_MIN_SPEED_M_S && slip < -ABS_SLIP_LIMIT;
        let share = if locking {
            (before - ABS_RELEASE_STEP).max(0.0)
        } else {
            (before + ABS_REAPPLY_STEP).min(1.0)
        };
        self.brake_share[index] = share;
        trace!(wheel = index, slip, road_speed_m_s, share, "ABS wheel slip");
        if before >= 1.0 && share < 1.0 {
            debug!(
                wheel = index,
                slip,
                road_speed_m_s,
                brake_share_before = before,
                brake_share = share,
                "ABS releases the brake of a locking wheel"
            );
        }
        share
    }

    /// The drive share for the next step from the slip of the fastest driven wheel. Above
    /// `ASR_SLIP_LIMIT` it loses `ASR_CUT_STEP` (not below `ASR_MIN_SHARE`), otherwise it regains
    /// `ASR_REAPPLY_STEP` up to full.
    pub fn drive_share_for(&mut self, worst_slip: f32) -> f32 {
        if !self.asr {
            self.drive_share = 1.0;
            return 1.0;
        }
        let before = self.drive_share;
        self.drive_share = if worst_slip > ASR_SLIP_LIMIT {
            (before - ASR_CUT_STEP).max(ASR_MIN_SHARE)
        } else {
            (before + ASR_REAPPLY_STEP).min(1.0)
        };
        trace!(worst_slip, drive_share = self.drive_share, "ASR drive slip");
        if before >= 1.0 && self.drive_share < 1.0 {
            debug!(
                worst_slip,
                drive_share = self.drive_share,
                "ASR cuts the drive torque of a spinning wheel"
            );
        }
        self.drive_share
    }
}
