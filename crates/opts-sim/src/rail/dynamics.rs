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
    fn script_forces_replace_the_constants_and_a_script_brake_holds_the_train() {
        let d = RailDynamics::tram(30_000.0);
        let pull = RailInput {
            script: Some(ScriptForces {
                traction_n: 15_000.0,
                brake_n: 0.0,
            }),
            ..Default::default()
        };
        let a = d.acceleration(0.1, 0.0, &pull);
        assert!((a - (15_000.0 - d.davis_a) / 30_000.0).abs() < 0.01, "{a}");
        let held = RailInput {
            script: Some(ScriptForces {
                traction_n: 5_000.0,
                brake_n: 20_000.0,
            }),
            ..Default::default()
        };
        assert_eq!(d.step(0.0, 0.0, &held, 0.1), 0.0);
        // Adhesion still caps what a script asks for.
        let huge = RailInput {
            script: Some(ScriptForces {
                traction_n: 1.0e7,
                brake_n: 0.0,
            }),
            ..Default::default()
        };
        assert!(d.acceleration(0.1, 0.0, &huge) < 3.0);
    }

    fn freight() -> (RailDynamics, RailDynamics) {
        // One 84 t locomotive and twenty 40 t wagons (884 t): what a long `.zug` is.
        let train = RailDynamics::for_consist(ConsistLoad {
            total_kg: 884_000.0,
            powered_kg: 84_000.0,
            braked_kg: 884_000.0,
        });
        (train, RailDynamics::tram(84_000.0))
    }

    #[test]
    fn a_single_body_consist_is_the_old_tram() {
        let a = RailDynamics::for_consist(ConsistLoad::single(30_000.0));
        let b = RailDynamics::tram(30_000.0);
        assert_eq!(a, b);
        assert_eq!(a.powered_mass_kg, 30_000.0);
        assert_eq!(a.braked_mass_kg, 30_000.0);
    }

    /// The force that produced `a` on `d` at speed `v`: mass times acceleration plus resistance.
    fn pull_n(d: &RailDynamics, a: f32, v: f32) -> f32 {
        a * d.mass_kg + d.davis_a + d.davis_b * v + d.davis_c * v * v
    }

    #[test]
    fn a_heavy_consist_accelerates_with_the_traction_of_its_powered_cars_only() {
        let (train, loco_alone) = freight();
        let a_train = train.acceleration(0.1, 0.0, &go());
        let a_loco = loco_alone.acceleration(0.1, 0.0, &go());
        // The pull is the locomotive's 1.1 N/kg x 84 t in both; only the mass differs.
        let f = 1.1 * 84_000.0;
        assert!((pull_n(&train, a_train, 0.1) / f - 1.0).abs() < 0.01);
        assert!((pull_n(&loco_alone, a_loco, 0.1) / f - 1.0).abs() < 0.01);
        // 884 t with rolling resistance (26 kN): about 0.075 m/s², a fraction of the lone loco's 1.1.
        assert!((0.06..0.09).contains(&a_train), "{a_train}");
        assert!(a_loco / a_train > 10.0, "{a_loco} / {a_train}");
    }

    #[test]
    fn acceleration_scales_with_one_over_total_mass_at_equal_traction() {
        let build = |total| {
            RailDynamics::for_consist(ConsistLoad {
                total_kg: total,
                powered_kg: 100_000.0,
                braked_kg: total,
            })
        };
        let (light, heavy) = (build(200_000.0), build(400_000.0));
        assert_eq!(light.max_traction_n, heavy.max_traction_n);
        let (a_l, a_h) = (
            light.acceleration(0.1, 0.0, &go()),
            heavy.acceleration(0.1, 0.0, &go()),
        );
        // Equal force on twice the mass: the force that accelerated them is the same within 2 %.
        let (f_l, f_h) = (pull_n(&light, a_l, 0.1), pull_n(&heavy, a_h, 0.1));
        assert!((f_l / f_h - 1.0).abs() < 0.02, "{f_l} vs {f_h}");
        assert!(a_l > 1.7 * a_h && a_l < 2.6 * a_h, "{a_l} vs {a_h}");
    }

    #[test]
    fn a_gradient_pulls_on_the_whole_train_but_traction_does_not_grow() {
        let (train, _) = freight();
        // 1 % uphill: 884 t x g x 0.01 = 87 kN, nearly all of the locomotive's 92 kN.
        let on_grade = train.acceleration(5.0, 0.01, &go());
        let flat = train.acceleration(5.0, 0.0, &go());
        assert!(on_grade < flat - 0.08, "{on_grade} vs {flat}");
    }

    #[test]
    fn braking_force_follows_the_braked_mass() {
        let all = RailDynamics::for_consist(ConsistLoad {
            total_kg: 400_000.0,
            powered_kg: 100_000.0,
            braked_kg: 400_000.0,
        });
        let head_only = RailDynamics::for_consist(ConsistLoad {
            total_kg: 400_000.0,
            powered_kg: 100_000.0,
            braked_kg: 100_000.0,
        });
        let brake = RailInput {
            brake: 1.0,
            ..Default::default()
        };
        let (a_all, a_head) = (
            all.acceleration(10.0, 0.0, &brake),
            head_only.acceleration(10.0, 0.0, &brake),
        );
        assert!(a_all < -1.2, "{a_all}");
        assert!((a_head / a_all - 0.25).abs() < 0.06, "{a_head} / {a_all}");
    }

    #[test]
    fn masses_are_clamped_into_a_sensible_order() {
        let d = RailDynamics::for_consist(ConsistLoad {
            total_kg: 10_000.0,
            powered_kg: 90_000.0,
            braked_kg: 1.0,
        });
        assert_eq!(d.mass_kg, 10_000.0);
        assert_eq!(d.powered_mass_kg, 10_000.0);
        assert_eq!(d.braked_mass_kg, 5_000.0);
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
