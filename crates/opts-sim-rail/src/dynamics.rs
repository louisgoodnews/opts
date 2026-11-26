//! Longitudinal dynamics of a rail vehicle on a gradient.
//!
//! Tractive effort follows a power-limited curve, the running resistance is the Davis form
//! `A + B·v + C·v²`, gradients act with `m·g·slope`, brakes are force limited and wheel/rail
//! adhesion caps both traction and braking. The constants are *assumed* typical values for light
//! rail (OMSI keeps its rail physics in scripts and constants, not in the `.bus`; see
//! `docs/src/simulation/rail-vehicles.md` §6): they give believable acceleration, they are not a
//! measurement of any particular vehicle.

/// Standard gravity (m/s²).
pub const G: f32 = 9.806_65;

/// Driver inputs for one step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RailInput {
    /// Traction demand `0..=1`.
    pub traction: f32,
    /// Service brake demand `0..=1`.
    pub brake: f32,
    /// Emergency brake.
    pub emergency: bool,
    /// Holding brake (parking brake); also brakes at standstill.
    pub hold: bool,
    /// Travel direction `+1`/`-1` selected by the driver (only matters at standstill).
    pub direction: i8,
    /// Forces the vehicle's own scripts hand over (`M_Wheel` at the wheel radius, `Brakeforce`):
    /// they replace the traction curve and the brake constants; adhesion still limits both.
    pub script: Option<ScriptForces>,
}

/// The traction and brake force of a consist as its scripts compute them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScriptForces {
    /// Tractive force at the rails (N, not negative).
    pub traction_n: f32,
    /// Brake force at the rails (N, not negative).
    pub brake_n: f32,
}

/// What a consist weighs and what acts on it: resistance and gradient act on the total mass,
/// brakes on the braked mass, traction and power on the powered mass (ADR-0013).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsistLoad {
    /// Mass of every car (kg).
    pub total_kg: f32,
    /// Mass of the cars that drive (the head and the cars with a driver workstation), kg.
    pub powered_kg: f32,
    /// Mass of the cars that brake (all of them), kg.
    pub braked_kg: f32,
}

impl ConsistLoad {
    /// A single body: everything counts as powered and braked.
    #[must_use]
    pub fn single(mass_kg: f32) -> Self {
        Self {
            total_kg: mass_kg,
            powered_kg: mass_kg,
            braked_kg: mass_kg,
        }
    }
}

/// Vehicle constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailDynamics {
    /// Total mass (kg).
    pub mass_kg: f32,
    /// Mass on driven axles (kg): limits traction through adhesion.
    pub powered_mass_kg: f32,
    /// Mass on braked axles (kg): limits braking through adhesion.
    pub braked_mass_kg: f32,
    /// Maximum tractive effort at low speed (N).
    pub max_traction_n: f32,
    /// Continuous traction power (W).
    pub power_w: f32,
    /// Service brake force (N).
    pub brake_n: f32,
    /// Emergency brake force (N).
    pub emergency_n: f32,
    /// Davis constant term (N).
    pub davis_a: f32,
    /// Davis linear term (N per m/s).
    pub davis_b: f32,
    /// Davis quadratic term (N per (m/s)²).
    pub davis_c: f32,
    /// Wheel/rail adhesion coefficient (dry steel ≈ 0.3, wet ≈ 0.15).
    pub adhesion: f32,
    /// Top speed (m/s).
    pub max_speed: f32,
}

impl RailDynamics {
    /// Typical light-rail tram of `mass_kg` (about 1.1 m/s² from rest, 1.3 m/s² service braking,
    /// 70 km/h).
    #[must_use]
    pub fn tram(mass_kg: f32) -> Self {
        Self::for_consist(ConsistLoad::single(mass_kg))
    }

    /// A consist: the tram constants per kilogram applied to the right mass for each effect.
    /// Running resistance (rolling part) and gradient use the total mass, traction and power the
    /// powered mass, service and emergency brakes the braked mass; the aerodynamic term is one
    /// frontal area whatever the length. Masses below 5 t are raised to 5 t.
    #[must_use]
    pub fn for_consist(load: ConsistLoad) -> Self {
        let m = load.total_kg.max(5_000.0);
        let p = load.powered_kg.clamp(5_000.0, m);
        let b = load.braked_kg.clamp(5_000.0, m);
        Self {
            mass_kg: m,
            powered_mass_kg: p,
            braked_mass_kg: b,
            max_traction_n: 1.1 * p,
            power_w: 14.0 * p, // ≈ 14 W per kg: 420 kW for 30 t
            brake_n: 1.3 * b,
            emergency_n: 2.6 * b,
            davis_a: 0.005 * m * G * 0.6,
            davis_b: 0.0,
            davis_c: 6.0,
            adhesion: 0.25,
            max_speed: 70.0 / 3.6,
        }
    }

    /// Net acceleration (m/s²) at `speed` (≥ 0, along the direction of travel) on `grade`
    /// (rise per metre, positive uphill in the direction of travel).
    #[must_use]
    pub fn acceleration(&self, speed: f32, grade: f32, input: &RailInput) -> f32 {
        let v = speed.max(0.0);
        let traction_limit = self.adhesion * self.powered_mass_kg * G;
        let brake_limit = self.adhesion * self.braked_mass_kg * G;
        let traction = if v >= self.max_speed {
            0.0
        } else if let Some(f) = input.script {
            f.traction_n.max(0.0).min(traction_limit)
        } else {
            (input.traction.clamp(0.0, 1.0)
                * self
                    .max_traction_n
                    .min(self.power_w / v.max(self.power_w / self.max_traction_n)))
            .min(traction_limit)
        };
        let brake = if let Some(f) = input.script {
            f.brake_n.max(0.0)
        } else if input.emergency {
            self.emergency_n
        } else {
            input.brake.clamp(0.0, 1.0) * self.brake_n
        }
        .min(brake_limit);
        let resist = self.davis_a + self.davis_b * v + self.davis_c * v * v;
        let slope = self.mass_kg * G * grade.atan().sin();
        (traction - brake - resist - slope) / self.mass_kg
    }

    /// One step: returns the new speed (≥ 0). Brakes and resistance never reverse the train.
    #[must_use]
    pub fn step(&self, speed: f32, grade: f32, input: &RailInput, dt: f32) -> f32 {
        let v = speed.max(0.0);
        if input.hold && input.traction <= 0.0 && input.script.is_none() {
            return (v - self.brake_n / self.mass_kg * dt).max(0.0); // holding brake: the braked mass's force on the whole train
        }
        let a = self.acceleration(v, grade, input);
        let mut next = v + a * dt;
        // Standing still on a gradient steeper than the resistance: rolls back is not modelled;
        // brakes and resistance cannot turn the speed negative.
        if next < 0.0 {
            next = 0.0;
        }
        next.min(self.max_speed)
    }
}

#[cfg(test)]
mod tests;
