//! Unit tests of `vehicle.surface`: the parameters, the range errors, the wash and the report.

use super::*;

#[test]
fn parameters_are_read_and_checked() {
    let r = parse(&json!({"dirt": 0.4, "wet": 1.0, "wiped": true})).unwrap();
    assert_eq!(
        r,
        Request {
            dirt: Some(0.4),
            wet: Some(1.0),
            wiped: true
        }
    );
    assert_eq!(parse(&json!({})).unwrap(), Request::default());
}

#[test]
fn values_outside_zero_to_one_are_refused_with_invalid_params() {
    for bad in [
        json!({"dirt": 1.5}),
        json!({"wet": -0.1}),
        json!({"dirt": "dirty"}),
    ] {
        let e = parse(&bad).unwrap_err();
        assert_eq!(e.code, code::INVALID_PARAMS);
    }
}

#[test]
fn a_wash_resets_the_dirt_and_sets_the_time() {
    let mut s = VehicleSurface::default();
    apply(
        &mut s,
        &Request {
            dirt: Some(0.7),
            ..Request::default()
        },
        0.0,
    );
    assert_eq!(s.dirt, 0.7);
    apply(
        &mut s,
        &Request {
            wiped: true,
            ..Request::default()
        },
        3600.0,
    );
    assert_eq!(s.dirt, 0.0);
    assert_eq!(s.wiped_at, Some(3600.0));
    // The same request sets and washes in one: the wash wins.
    apply(
        &mut s,
        &Request {
            dirt: Some(0.5),
            wiped: true,
            ..Request::default()
        },
        7.0,
    );
    assert_eq!(s.dirt, 0.0);
}

#[test]
fn wetness_is_set_and_the_snow_is_reported() {
    let mut s = VehicleSurface::default();
    apply(
        &mut s,
        &Request {
            wet: Some(0.25),
            ..Request::default()
        },
        0.0,
    );
    let d = describe(&s, 0.5);
    assert_eq!(d["wet"].as_f64().map(|x| (x * 100.0).round()), Some(25.0));
    assert_eq!(d["wiped_at"], Value::Null);
    assert_eq!(d["snow"].as_f64().map(|x| (x * 100.0).round()), Some(50.0));
}
