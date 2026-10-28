//! Unit tests for `super` (`traits.rs`).

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
    IdmParams::from_traits(t, &Calibration::default(), 0.5)
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
        (p.speed_factor - 1.0).abs() < 1e-9 && (p.lat_accel - 2.5).abs() < 1e-9,
        "roll 0.5 is the middle"
    );
    assert!(
        (p.desired_speed - 130.0 / 3.6).abs() < 1e-9,
        "default top speed"
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
fn the_top_speed_caps_and_the_limit_is_applied_per_lane() {
    // Semantics changed (C3): the lane limit is no longer folded in at spawn; the driver
    // applies its factor to each lane's limit, under the vehicle's top speed.
    let scooter = VehicleTraits {
        top_speed: Some(15.0 / 3.6),
        ..car()
    };
    let p = params(&scooter);
    assert!(p.lane_speed(Some(50.0 / 3.6), 50.0 / 3.6) <= 15.0 / 3.6 + 1e-9);
    let fast = IdmParams::from_traits(&car(), &Calibration::default(), 1.0);
    assert!((fast.lane_speed(Some(8.0), 50.0 / 3.6) - 8.0 * 1.1).abs() < 1e-9);
    let free = params(&car());
    assert!(
        (free.lane_speed(None, 50.0 / 3.6) - 50.0 / 3.6).abs() < 1e-9,
        "open road speed, not the 160 km/h top speed"
    );
    assert!((free.desired_speed - 160.0 / 3.6).abs() < 1e-9);
    assert_eq!(params(&lorry()).lat_accel, 2.0);
}

#[test]
fn the_rowdy_draw_stays_inside_its_range() {
    let cal = Calibration::default();
    let t = VehicleTraits {
        rowdy: (0.4, 0.6),
        ..car()
    };
    let lowest = IdmParams::from_traits(&t, &cal, 0.0).speed_factor;
    let highest = IdmParams::from_traits(&t, &cal, 0.999).speed_factor;
    assert!(
        lowest >= 0.98 - 1e-9 && highest <= 1.02 + 1e-9,
        "{lowest} {highest}"
    );
    let fixed = VehicleTraits {
        rowdy: (0.0, 0.0),
        ..car()
    };
    let a = IdmParams::from_traits(&fixed, &cal, 0.1).speed_factor;
    let b = IdmParams::from_traits(&fixed, &cal, 0.9).speed_factor;
    assert_eq!(a, b);
    assert!((a - 0.9).abs() < 1e-9, "rowdy 0 is the calmest driver");
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
