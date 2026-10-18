//! The service brake of a road vehicle: a force per axle sized for the axle's rated load, a
//! pneumatic build-up and release, and what the tyres can carry (M-0073).
//!
//! The brakes are sized for the loads the vehicle files rate their axles for
//! (`achse_maxforce`): an axle brakes with at most [`BRAKE_FACTOR`] times its rated load, so a
//! vehicle that carries its rated load decelerates at most at `BRAKE_FACTOR · g` however good the
//! road is, and one that carries less is limited by the road instead. All axles are applied by one
//! pressure (no brake-force distribution valve, no ABS: an axle whose force exceeds what its tyres
//! carry (`ROAD_FRICTION · load`) is held at that limit, which is a lock-up baseline). Where the
//! vehicle's own scripts drive the brakes (`Brakeforce`, `Axle_Brakeforce_*`), they stay
//! authoritative and only the tyre limit applies (see `VehicleSimulation::step`).

use serde::{Deserialize, Serialize};

/// Brake force of an axle at full pressure as a fraction of its rated load: 0.55 gives
/// `0.55 g ≈ 5.4 m/s²` for a vehicle at its rated load (assumed ❓, the files state no brake data).
pub const BRAKE_FACTOR: f32 = 0.55;
/// Time constant of the pressure building up in the brake chambers of a heavy (air-braked)
/// vehicle (s).
pub const APPLY_TAU_S: f32 = 0.30;
/// Time constant of the pressure falling when the pedal is released (s).
pub const RELEASE_TAU_S: f32 = 0.20;
/// Time constants of a light (hydraulically braked) vehicle: build-up and release (s).
pub const LIGHT_APPLY_TAU_S: f32 = 0.12;
/// See [`LIGHT_APPLY_TAU_S`].
pub const LIGHT_RELEASE_TAU_S: f32 = 0.08;
/// Total rated axle load (N) below which a vehicle counts as light: a car or a van.
pub const LIGHT_RATED_N: f32 = 50_000.0;

/// The service brake's pressure and the forces it produced in the last step.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BrakeSystem {
    /// Chamber pressure as a fraction of the full pressure (0 to 1).
    pub pressure: f32,
    /// Time constant of the build-up (s); zero reads as [`APPLY_TAU_S`] (set by
    /// [`BrakeSystem::size_for`]).
    #[serde(default)]
    pub apply_tau_s: f32,
    /// Time constant of the release (s); zero reads as [`RELEASE_TAU_S`].
    #[serde(default)]
    pub release_tau_s: f32,
    /// Force of each axle in the last step (N), in the order the axles were given.
    pub axle_forces_n: Vec<f32>,
    /// Whether the axle was held at the limit of its tyres in the last step.
    pub axle_limited: Vec<bool>,
}

impl BrakeSystem {
    /// Chooses the time constants by the total rated load of the vehicle: hydraulic brakes (a car,
    /// a van: below [`LIGHT_RATED_N`]) are quicker than air brakes ❓.
    pub fn size_for(&mut self, total_rated_n: f32) {
        let light = total_rated_n < LIGHT_RATED_N;
        self.apply_tau_s = if light {
            LIGHT_APPLY_TAU_S
        } else {
            APPLY_TAU_S
        };
        self.release_tau_s = if light {
            LIGHT_RELEASE_TAU_S
        } else {
            RELEASE_TAU_S
        };
    }

    /// Advances the pressure towards the pedal position and returns the total brake force (N).
    ///
    /// `capacities_n` is the force of each axle at full pressure, `limits_n` what its tyres can
    /// carry now (`ROAD_FRICTION · load`).
    pub fn step(&mut self, pedal: f32, capacities_n: &[f32], limits_n: &[f32], dt: f32) -> f32 {
        let target = pedal.clamp(0.0, 1.0);
        let tau = if target > self.pressure {
            if self.apply_tau_s > 0.0 {
                self.apply_tau_s
            } else {
                APPLY_TAU_S
            }
        } else if self.release_tau_s > 0.0 {
            self.release_tau_s
        } else {
            RELEASE_TAU_S
        };
        self.pressure += (target - self.pressure) * (1.0 - (-dt.max(0.0) / tau).exp());
        if (self.pressure - target).abs() < 1e-4 {
            self.pressure = target;
        }
        self.axle_forces_n.clear();
        self.axle_limited.clear();
        let mut total = 0.0;
        for (i, cap) in capacities_n.iter().enumerate() {
            let want = cap * self.pressure;
            let limit = limits_n.get(i).copied().unwrap_or(f32::INFINITY).max(0.0);
            let force = want.min(limit);
            self.axle_forces_n.push(force);
            self.axle_limited.push(want > limit);
            total += force;
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pressure_builds_up_and_falls_with_its_time_constants() {
        let mut b = BrakeSystem::default();
        let caps = [60_000.0, 60_000.0];
        let lim = [1.0e9, 1.0e9];
        // After one apply time constant the pressure is 63 % of the pedal.
        for _ in 0..30 {
            b.step(1.0, &caps, &lim, 0.01);
        }
        assert!((b.pressure - 0.632).abs() < 0.01, "{}", b.pressure);
        for _ in 0..600 {
            b.step(1.0, &caps, &lim, 0.01);
        }
        assert_eq!(b.pressure, 1.0);
        // Released: falling with the shorter constant.
        for _ in 0..20 {
            b.step(0.0, &caps, &lim, 0.01);
        }
        assert!((b.pressure - 0.368).abs() < 0.01, "{}", b.pressure);
    }

    #[test]
    fn an_axle_is_held_at_what_its_tyres_carry() {
        let mut b = BrakeSystem {
            pressure: 1.0,
            ..BrakeSystem::default()
        };
        let total = b.step(1.0, &[60_000.0, 60_000.0], &[20_000.0, 90_000.0], 0.01);
        assert_eq!(b.axle_forces_n, vec![20_000.0, 60_000.0]);
        assert_eq!(b.axle_limited, vec![true, false]);
        assert_eq!(total, 80_000.0);
    }
}
