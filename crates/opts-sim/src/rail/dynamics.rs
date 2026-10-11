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
}

/// Vehicle constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailDynamics {
    /// Total mass (kg).
    pub mass_kg: f32,
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
        let m = mass_kg.max(5_000.0);
        Self {
            mass_kg: m,
            max_traction_n: 1.1 * m,
            power_w: 14.0 * m * 1.0, // ≈ 14 W per kg: 420 kW for 30 t
            brake_n: 1.3 * m,
            emergency_n: 2.6 * m,
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
        let adhesion_limit = self.adhesion * self.mass_kg * G;
        let traction = if v >= self.max_speed {
            0.0
        } else {
            (input.traction.clamp(0.0, 1.0)
                * self
                    .max_traction_n
                    .min(self.power_w / v.max(self.power_w / self.max_traction_n)))
            .min(adhesion_limit)
        };
        let brake = if input.emergency {
            self.emergency_n
        } else {
            input.brake.clamp(0.0, 1.0) * self.brake_n
        }
        .min(adhesion_limit);
        let resist = self.davis_a + self.davis_b * v + self.davis_c * v * v;
        let slope = self.mass_kg * G * grade.atan().sin();
        (traction - brake - resist - slope) / self.mass_kg
    }

    /// One step: returns the new speed (≥ 0). Brakes and resistance never reverse the train.
    #[must_use]
    pub fn step(&self, speed: f32, grade: f32, input: &RailInput, dt: f32) -> f32 {
        let v = speed.max(0.0);
        if input.hold && input.traction <= 0.0 {
            return (v - self.brake_n / self.mass_kg * dt).max(0.0);
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
mod tests {
    use super::*;

    fn go() -> RailInput {
        RailInput {
            traction: 1.0,
            direction: 1,
            ..Default::default()
        }
    }

    #[test]
    fn a_tram_accelerates_to_about_one_metre_per_second_squared_and_saturates() {
        let d = RailDynamics::tram(30_000.0);
        let a0 = d.acceleration(0.1, 0.0, &go());
        assert!((0.9..1.15).contains(&a0), "{a0}");
        let mut v = 0.0;
        for _ in 0..4000 {
            v = d.step(v, 0.0, &go(), 0.05);
        }
        assert!((v - d.max_speed).abs() < 1.0, "{v}");
    }

    #[test]
    fn braking_stops_in_the_distance_physics_predicts() {
        let d = RailDynamics::tram(30_000.0);
        let (mut v, mut x) = (50.0 / 3.6, 0.0);
        let input = RailInput {
            brake: 1.0,
            ..Default::default()
        };
        while v > 0.0 {
            x += v * 0.01;
            v = d.step(v, 0.0, &input, 0.01);
        }
        // v²/(2a) with a ≈ 1.3 m/s² plus resistance: 74 m or a little less.
        assert!((60.0..76.0).contains(&x), "{x}");
    }

    #[test]
    fn an_uphill_grade_slows_and_adhesion_limits_a_wet_start() {
        let d = RailDynamics::tram(30_000.0);
        assert!(d.acceleration(5.0, 0.05, &go()) < d.acceleration(5.0, 0.0, &go()) - 0.4);
        let wet = RailDynamics {
            adhesion: 0.08,
            ..d
        };
        assert!(wet.acceleration(0.1, 0.0, &go()) < 0.8);
    }

    #[test]
    fn resistance_and_brakes_never_make_the_speed_negative() {
        let d = RailDynamics::tram(30_000.0);
        let v = d.step(
            0.05,
            0.0,
            &RailInput {
                emergency: true,
                ..Default::default()
            },
            0.5,
        );
        assert_eq!(v, 0.0);
    }
}
