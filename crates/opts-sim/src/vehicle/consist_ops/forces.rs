//! Consist forces: the static friction loads over all units and the force a joint carries.

use super::*;

impl VehicleSimulation {
    /// Summed static normal force `(driven axles, braked axles)` over all units (N): what the
    /// tyres can transmit is `ROAD_FRICTION` times these. The chassis' axles carry its own mass
    /// (distributed by the suspension model), each following unit its own; every axle brakes.
    #[must_use]
    pub fn friction_loads_n(&self) -> (f32, f32) {
        let positions: Vec<f32> = self.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let head = self
            .suspension
            .compute_static_loads(&positions, self.chassis.total_mass_kg);
        // A head without any axle marked driven counts as driven on all of them (the file does
        // not say; do not take away all traction).
        let head_has_driven = self.axles.iter().any(|a| a.config.is_driven);
        let mut axles: Vec<(UnitAxle, f32)> = self
            .axles
            .iter()
            .zip(head)
            .map(|(a, n)| {
                (
                    UnitAxle {
                        longitudinal_m: a.config.longitudinal_m,
                        is_driven: a.config.is_driven || !head_has_driven,
                        rated_n: a.config.max_force_n,
                    },
                    n,
                )
            })
            .collect();
        for unit in &self.trailer_loads {
            axles.extend(unit.axles.iter().copied().zip(unit.axle_loads_n()));
        }
        driven_and_braked_load_n(&axles)
    }

    /// Force the joint of consist unit `k` carries (N, report only; nothing is fed back): the
    /// mass behind it accelerated with the chassis plus its rolling resistance,
    /// `F = Σ_{j≥k} mⱼ (a + sign(v)·Crr·g)`. `None` for a missing unit.
    #[must_use]
    pub fn hitch_force_n(&self, k: usize) -> Option<f32> {
        let rest = self
            .trailer_loads
            .get(k..)
            .filter(|rest| !rest.is_empty())?;
        let people: f32 = self
            .unit_passenger_mass_kg
            .iter()
            .skip(k)
            .take(rest.len())
            .map(|m| m.max(0.0))
            .sum();
        let behind: f32 = rest.iter().map(|u| u.mass_kg).sum::<f32>() + people;
        let v = self.chassis.forward_speed_m_s();
        let direction = if v.abs() > 0.01 { v.signum() } else { 0.0 };
        let a = self.chassis.local_acceleration.y;
        Some(behind * (a + direction * drivetrain::ROLLING_RESISTANCE_COEFF * GRAVITY))
    }
}
