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

mod speed_and_class;
