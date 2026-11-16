//! Consist mass: the head and following units, the people on board and the body payload.

use super::*;

impl VehicleSimulation {
    /// Mass of the whole consist the longitudinal dynamics act on (kg): the chassis' own mass
    /// plus the following units' (design D2, `Σ mᵢ`) plus the people on board.
    #[must_use]
    pub fn consist_mass_kg(&self) -> f32 {
        self.chassis.total_mass_kg
            + self.trailer_loads.iter().map(|u| u.mass_kg).sum::<f32>()
            + self.passenger_mass_kg
            + self.following_passenger_mass_kg()
    }

    /// Mass of all people on board the consist (kg), head and following units; the steering
    /// authority falls with it (M-0124).
    #[must_use]
    pub fn passenger_load_kg(&self) -> f32 {
        self.passenger_mass_kg.max(0.0) + self.following_passenger_mass_kg()
    }

    /// Mass of the people on board the following units (kg); entries without a unit do not
    /// count.
    #[must_use]
    pub fn following_passenger_mass_kg(&self) -> f32 {
        self.unit_passenger_mass_kg
            .iter()
            .take(self.trailer_loads.len())
            .map(|m| m.max(0.0))
            .sum()
    }

    /// Sets the mass of the people on board each unit of the consist (kg; entry 0 is the head,
    /// entry `k` the following unit `k - 1`). Units beyond the list carry nobody, entries beyond
    /// the units are dropped (M-0124).
    pub fn set_unit_passenger_masses(&mut self, masses_kg: &[f32]) {
        self.passenger_mass_kg = masses_kg.first().copied().unwrap_or(0.0);
        let followers = self.trailer_loads.len();
        let rest = masses_kg.get(1..).unwrap_or(&[]);
        self.unit_passenger_mass_kg = (0..followers)
            .map(|k| rest.get(k).copied().unwrap_or(0.0))
            .collect();
    }

    /// Gives the body the weight of the people on board, shared over the head's axles as a load
    /// in the middle of the axle span would be (M-0124). A no-op while it is unchanged.
    pub(in crate::vehicle) fn sync_payload(&mut self) {
        let Some(body) = self.body.as_mut() else {
            return;
        };
        let weight = self.passenger_mass_kg.max(0.0) * 9.81;
        if (body.payload_n() - weight).abs() < 1.0 {
            return;
        }
        let positions: Vec<f32> = self.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let (Some(lo), Some(hi)) = (
            positions.iter().copied().reduce(f32::min),
            positions.iter().copied().reduce(f32::max),
        ) else {
            return;
        };
        // `compute_static_loads` takes at least 100 kg: share a reference mass, scale to the load.
        let reference = 1_000.0;
        let model = crate::vehicle::suspension::SuspensionModel::new(
            self.suspension.cg_height_m,
            0.5 * (lo + hi),
        );
        let loads: Vec<f32> = model
            .compute_static_loads(&positions, reference)
            .into_iter()
            .map(|l| l * (weight / (reference * 9.81)))
            .collect();
        body.set_payload(&loads);
    }

    /// Mass of the head unit alone (kg): the chassis' own mass.
    #[must_use]
    pub fn head_mass_kg(&self) -> f32 {
        self.chassis.total_mass_kg
    }

    /// Mass standing on driven axles over all units (kg), `friction_loads_n().0 / g`.
    #[must_use]
    pub fn driven_mass_kg(&self) -> f32 {
        self.friction_loads_n().0 / GRAVITY
    }
}
