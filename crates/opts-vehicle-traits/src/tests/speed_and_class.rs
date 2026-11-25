//! Tests of the top speed, the rowdy draw and the vehicle class.

use super::*;

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
