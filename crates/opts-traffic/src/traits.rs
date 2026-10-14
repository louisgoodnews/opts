//! From what a vehicle file says to how the car-following model drives it.
//!
//! [`VehicleTraits`] holds plain numbers (the host reads them from the vehicle's files, see
//! `opts_core::formats::vehicles::AiProfile`); [`IdmParams::from_traits`] maps them to
//! [`IdmParams`] with the constants of a [`Calibration`]. The crate stays free of files and
//! Bevy. Design and open questions: `docs/src/architecture/ai-vehicle-config.md`.
//!
//! The mapping is deliberately simple and clamped. The drive and brake constants are the ones
//! OMSI's own AI scripts use (`M_Wheel = Throttle · AI_M_gas_max · (1 − v / AI_v_max_erreich)`,
//! `Brakeforce = Brake · AI_F_brems_max`); how OMSI's engine-side driver uses them is not
//! measured ❓, so every number that is a *behaviour* choice lives in [`Calibration`].

use crate::idm::{IdmParams, VehicleClass};

/// Kind of AI vehicle (`[ai_veh_type]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleKind {
    /// A passenger car.
    Car,
    /// A cab.
    Cab,
    /// A bus.
    Bus,
    /// A truck.
    Truck,
}

/// What the files declare about one vehicle. `None` means "not declared": the mapping then
/// uses the defaults of the vehicle's class.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleTraits {
    /// `[ai_veh_type]`, when the file has it.
    pub kind: Option<VehicleKind>,
    /// The vehicle has a passenger cabin (a bus when `kind` is unknown).
    pub has_cabin: bool,
    /// Body length in metres.
    pub length: f64,
    /// Mass in kilograms.
    pub mass_kg: Option<f64>,
    /// Top speed in m/s (`AI_v_max`).
    pub top_speed: Option<f64>,
    /// Drive torque at full throttle (`AI_M_gas_max`).
    pub drive_torque: Option<f64>,
    /// Radius of the driven wheels in metres.
    pub wheel_radius: Option<f64>,
    /// Brake force at full brake in newtons (`AI_F_brems_max`).
    pub brake_force: Option<f64>,
    /// Brake multiplier (`[ai_brakeperformance]`), `1.0` when neutral.
    pub brake_performance: f64,
    /// Range the per-vehicle "rowdy" draw lies in (`[rowdy_factor]`), within `0..=1`.
    pub rowdy: (f64, f64),
}

impl Default for VehicleTraits {
    /// A vehicle the files say nothing about: a 4.4 m car on the class defaults.
    fn default() -> Self {
        Self {
            kind: None,
            has_cabin: false,
            length: 4.4,
            mass_kg: None,
            top_speed: None,
            drive_torque: None,
            wheel_radius: None,
            brake_force: None,
            brake_performance: 1.0,
            rowdy: (0.0, 1.0),
        }
    }
}

impl VehicleTraits {
    /// The vehicle class for lane restrictions: from `kind` when known, else a bus when it has
    /// a passenger cabin, a truck when it is longer than 7.5 m, otherwise a car.
    #[must_use]
    pub fn class(&self) -> VehicleClass {
        match self.kind {
            Some(VehicleKind::Car | VehicleKind::Cab) => VehicleClass::Car,
            Some(VehicleKind::Bus) => VehicleClass::Bus,
            Some(VehicleKind::Truck) => VehicleClass::Truck,
            None if self.has_cabin => VehicleClass::Bus,
            None if self.length > 7.5 => VehicleClass::Truck,
            None => VehicleClass::Car,
        }
    }
}

/// The behaviour constants of the mapping: choices about how a driver uses a vehicle, not data
/// of the vehicle. The defaults reproduce the literature values the traffic model has always
/// used for a typical car and lorry and are ❓ until measured against OMSI (Q-AI01).
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    /// Free-road speed (m/s) where a lane has no limit, before the per-vehicle draw.
    pub open_road_speed: f64,
    /// Half the width of the desired-speed spread around `open_road_speed` (`0.1` = ±10 %).
    pub speed_spread: f64,
    /// Fraction of the available drive force a driver uses for normal acceleration.
    pub accel_fraction: f64,
    /// Fraction of the available brake force a driver uses for comfortable braking.
    pub comfort_brake_fraction: f64,
    /// Wheel radius used when the file gives none: cars, then heavy vehicles (m).
    pub default_wheel_radius: (f64, f64),
    /// Range of normal acceleration, m/s².
    pub accel_range: (f64, f64),
    /// Range of comfortable deceleration, m/s².
    pub comfort_range: (f64, f64),
    /// Range of the hardest deceleration, m/s² (the tyres' grip bounds the top).
    pub max_decel_range: (f64, f64),
    /// Time headway of cars and cabs, then of buses and trucks (s).
    pub headway: (f64, f64),
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            open_road_speed: 50.0 / 3.6,
            speed_spread: 0.1,
            accel_fraction: 0.55,
            comfort_brake_fraction: 0.15,
            default_wheel_radius: (0.31, 0.5),
            accel_range: (0.5, 3.5),
            comfort_range: (1.0, 3.0),
            max_decel_range: (4.0, 9.0),
            headway: (1.5, 1.7),
        }
    }
}

fn clamp(v: f64, (lo, hi): (f64, f64)) -> f64 {
    v.clamp(lo, hi)
}

impl IdmParams {
    /// The driving parameters of the vehicle described by `traits`.
    ///
    /// `roll` is a uniform draw in `[0, 1)` that picks this vehicle's place in its rowdy range
    /// (the caller owns the random generator, so runs stay reproducible); `lane_limit` is the
    /// speed limit (m/s) of the lane it is placed on. Fields the traits do not give keep the
    /// defaults of the vehicle's class ([`IdmParams::default`] for cars, [`IdmParams::heavy`]
    /// otherwise).
    #[must_use]
    pub fn from_traits(
        traits: &VehicleTraits,
        cal: &Calibration,
        roll: f64,
        lane_limit: Option<f64>,
    ) -> Self {
        let class = traits.class();
        let heavy = class != VehicleClass::Car;
        let mut p = if heavy {
            Self::heavy()
        } else {
            Self::default()
        };
        p.class = class;
        p.length = traits.length;
        p.time_headway = if heavy { cal.headway.1 } else { cal.headway.0 };

        // Desired speed: the open-road speed, spread by this driver's rowdiness, never above the
        // vehicle's own top speed or the lane's limit.
        let (lo, hi) = traits.rowdy;
        let rowdy = (lo + (hi - lo) * roll).clamp(0.0, 1.0);
        let mut v0 = cal.open_road_speed * (1.0 + cal.speed_spread * (2.0 * rowdy - 1.0));
        if let Some(top) = traits.top_speed {
            v0 = v0.min(top);
        }
        if let Some(limit) = lane_limit {
            v0 = v0.min(limit);
        }
        p.desired_speed = v0.max(0.5);

        if let (Some(torque), Some(mass)) = (traits.drive_torque, traits.mass_kg) {
            let radius = traits.wheel_radius.unwrap_or(if heavy {
                cal.default_wheel_radius.1
            } else {
                cal.default_wheel_radius.0
            });
            let a = cal.accel_fraction * torque / (radius.max(0.1) * mass.max(1.0));
            p.max_accel = clamp(a, cal.accel_range);
        }
        if let (Some(force), Some(mass)) = (traits.brake_force, traits.mass_kg) {
            let per_kg = force * traits.brake_performance.max(0.1) / mass.max(1.0);
            p.comfort_decel = clamp(cal.comfort_brake_fraction * per_kg, cal.comfort_range);
            p.max_decel = clamp(per_kg, cal.max_decel_range);
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car() -> VehicleTraits {
        VehicleTraits {
            kind: Some(VehicleKind::Car),
            length: 4.1,
            mass_kg: Some(1300.0),
            top_speed: Some(160.0 / 3.6),
            drive_torque: Some(1000.0),
            wheel_radius: Some(0.325),
            brake_force: Some(18_000.0),
            ..VehicleTraits::default()
        }
    }

    fn lorry() -> VehicleTraits {
        VehicleTraits {
            kind: Some(VehicleKind::Truck),
            length: 11.0,
            mass_kg: Some(14_000.0),
            top_speed: Some(90.0 / 3.6),
            drive_torque: Some(8000.0),
            wheel_radius: Some(0.5),
            brake_force: Some(80_000.0),
            ..VehicleTraits::default()
        }
    }

    fn params(t: &VehicleTraits) -> IdmParams {
        IdmParams::from_traits(t, &Calibration::default(), 0.5, None)
    }

    #[test]
    fn nothing_declared_gives_the_class_defaults() {
        let p = params(&VehicleTraits::default());
        let d = IdmParams::default();
        assert_eq!(
            (p.max_accel, p.comfort_decel, p.max_decel),
            (d.max_accel, d.comfort_decel, d.max_decel)
        );
        assert!(
            (p.desired_speed - d.desired_speed).abs() < 1e-9,
            "roll 0.5 is the middle"
        );
        let heavy = VehicleTraits {
            length: 12.0,
            ..VehicleTraits::default()
        };
        let h = params(&heavy);
        let hd = IdmParams::heavy();
        assert_eq!(
            (h.max_accel, h.comfort_decel, h.class),
            (hd.max_accel, hd.comfort_decel, VehicleClass::Truck)
        );
    }

    #[test]
    fn a_typical_car_stays_near_the_old_constants() {
        let p = params(&car());
        assert!((p.max_accel - 1.4).abs() < 0.2, "{}", p.max_accel);
        assert!((p.comfort_decel - 2.0).abs() < 0.3, "{}", p.comfort_decel);
        assert_eq!(p.max_decel, 9.0);
        assert_eq!(p.class, VehicleClass::Car);
    }

    #[test]
    fn heavier_means_slower_and_more_brake_force_means_harder() {
        let (c, l) = (params(&car()), params(&lorry()));
        assert!(
            l.max_accel < c.max_accel,
            "{} vs {}",
            l.max_accel,
            c.max_accel
        );
        assert!(l.max_decel < c.max_decel && l.comfort_decel < c.comfort_decel);
        let heavier = VehicleTraits {
            mass_kg: Some(2600.0),
            ..car()
        };
        assert!(params(&heavier).max_accel < c.max_accel);
        let stronger = VehicleTraits {
            brake_force: Some(40_000.0),
            mass_kg: Some(6000.0),
            ..lorry()
        };
        let weaker = VehicleTraits {
            brake_force: Some(20_000.0),
            ..stronger.clone()
        };
        assert!(params(&stronger).max_decel > params(&weaker).max_decel);
        let better = VehicleTraits {
            brake_performance: 1.3,
            ..weaker.clone()
        };
        assert!(params(&better).max_decel > params(&weaker).max_decel);
    }

    #[test]
    fn clamps_hold_at_extreme_constants() {
        let wild = VehicleTraits {
            drive_torque: Some(1.0e9),
            brake_force: Some(1.0e9),
            ..car()
        };
        let p = params(&wild);
        assert_eq!((p.max_accel, p.max_decel, p.comfort_decel), (3.5, 9.0, 3.0));
        let weak = VehicleTraits {
            drive_torque: Some(1.0),
            brake_force: Some(1.0),
            ..car()
        };
        let p = params(&weak);
        assert_eq!((p.max_accel, p.max_decel, p.comfort_decel), (0.5, 4.0, 1.0));
    }

    #[test]
    fn top_speed_and_lane_limit_cap_the_desired_speed() {
        let scooter = VehicleTraits {
            top_speed: Some(15.0 / 3.6),
            ..car()
        };
        assert!(params(&scooter).desired_speed <= 15.0 / 3.6 + 1e-9);
        let limited = IdmParams::from_traits(&car(), &Calibration::default(), 1.0, Some(8.0));
        assert!((limited.desired_speed - 8.0).abs() < 1e-9);
        let free = IdmParams::from_traits(&car(), &Calibration::default(), 0.5, None);
        assert!(
            (free.desired_speed - 50.0 / 3.6).abs() < 1e-9,
            "open road speed, not the 160 km/h top speed"
        );
    }

    #[test]
    fn the_rowdy_draw_stays_inside_its_range() {
        let cal = Calibration::default();
        let t = VehicleTraits {
            rowdy: (0.4, 0.6),
            ..car()
        };
        let lowest = IdmParams::from_traits(&t, &cal, 0.0, None).desired_speed;
        let highest = IdmParams::from_traits(&t, &cal, 0.999, None).desired_speed;
        let base = cal.open_road_speed;
        assert!(
            lowest >= base * 0.98 - 1e-9 && highest <= base * 1.02 + 1e-9,
            "{lowest} {highest}"
        );
        let fixed = VehicleTraits {
            rowdy: (0.0, 0.0),
            ..car()
        };
        let a = IdmParams::from_traits(&fixed, &cal, 0.1, None).desired_speed;
        let b = IdmParams::from_traits(&fixed, &cal, 0.9, None).desired_speed;
        assert_eq!(a, b);
        assert!(
            (a - base * 0.9).abs() < 1e-9,
            "rowdy 0 is the calmest driver"
        );
    }

    #[test]
    fn class_follows_the_kind_then_the_cabin_then_the_length() {
        let mut t = VehicleTraits::default();
        assert_eq!(t.class(), VehicleClass::Car);
        t.length = 9.0;
        assert_eq!(t.class(), VehicleClass::Truck);
        t.has_cabin = true;
        assert_eq!(t.class(), VehicleClass::Bus);
        t.kind = Some(VehicleKind::Cab);
        assert_eq!(
            t.class(),
            VehicleClass::Car,
            "an explicit kind wins over the guess"
        );
        assert_eq!(params(&lorry()).time_headway, 1.7);
        assert_eq!(params(&car()).time_headway, 1.5);
    }
}
