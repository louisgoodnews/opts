//! The traffic traits of a vehicle type: the AI profile turned into the simulation's numbers.

use opts_traffic::{VehicleClass, VehicleTraits};
use super::*;

impl Variant {
    /// The traits the traffic model needs, from the profile of the head, the measured size of
    /// every unit (when known) and the masses and brake forces of all units.
    pub fn traits(&self) -> VehicleTraits {
        let head = &self.units[0].profile;
        let length: f64 = if self.units.iter().all(|u| u.size.is_some()) {
            self.units
                .iter()
                .filter_map(|u| u.size)
                .map(|(len, _)| f64::from(len))
                .sum()
        } else {
            f64::from(head.body_length_m.unwrap_or(4.4))
        };
        let masses: Option<Vec<f64>> = self
            .units
            .iter()
            .map(|u| u.profile.mass_kg.map(f64::from))
            .collect();
        let mass_kg = masses
            .as_ref()
            .map(|m| m.iter().sum())
            .or(head.mass_kg.map(f64::from));
        let brakes: Option<Vec<f64>> = self
            .units
            .iter()
            .map(|u| {
                u.response
                    .as_ref()
                    .map(|r| f64::from(r.brake_force))
                    .filter(|b| *b > 0.0)
                    .or(u.profile.brake_max_n.map(f64::from))
            })
            .collect();
        VehicleTraits {
            kind: head.kind.map(vehicle_kind),
            has_cabin: head.has_cabin,
            length,
            mass_kg,
            top_speed: head.top_speed_ms.map(f64::from),
            drive_torque: self.units[0]
                .response
                .as_ref()
                .map(|r| f64::from(r.peak_torque()))
                .filter(|t| *t > 0.0)
                .or(head.gas_max.map(f64::from)),
            wheel_radius: head.wheel_radius_m.map(f64::from),
            brake_force: brakes
                .map(|b| b.iter().sum())
                .or(head.brake_max_n.map(f64::from)),
            brake_performance: f64::from(head.brake_performance),
            rowdy: (f64::from(head.rowdy.0), f64::from(head.rowdy.1)),
            // The front axle sits `front_offset − y` behind the bumper; the wheelbase is the
            // head's own axle span (trailers follow through their joints).
            front_overhang: self.units[0]
                .size
                .zip(self.units[0].front_axle_y)
                .map(|((_, front), y)| f64::from(front - y)),
            wheelbase: head.wheelbase_m.map(f64::from),
        }
    }

    /// Class for lane restrictions (known before the meshes are measured).
    pub fn class(&self) -> VehicleClass {
        self.traits().class()
    }
}
