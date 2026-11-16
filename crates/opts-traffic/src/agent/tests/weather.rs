//! Weather in the traffic world (M-0117): grip and fog as the drivers feel them.

use super::*;
use crate::idm::IdmParams;
use crate::weather::grip_adjusted;

/// Mean speed (m/s) of a lone car over the second minute on a straight road with `grip`.
fn mean_speed_after_a_minute(grip: f64) -> f64 {
    let mut w = world(&[straight(0.0, 3000.0)]);
    w.set_weather(grip, f64::INFINITY);
    let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
    let (mut sum, mut n) = (0.0, 0);
    for k in 0..1200 {
        w.step(0.1);
        if k >= 600 {
            sum += w.agent(id).unwrap().speed;
            n += 1;
        }
    }
    sum / f64::from(n)
}

#[test]
fn snow_lowers_the_mean_speed_by_more_than_ten_percent() {
    let dry = mean_speed_after_a_minute(1.0);
    let snow = mean_speed_after_a_minute(0.25 / 0.85);
    assert!(dry > 10.0, "dry {dry}");
    assert!(snow <= 0.9 * dry, "snow {snow} vs dry {dry}");
}

#[test]
fn a_driver_never_brakes_harder_than_the_reduced_limit() {
    let grip = 0.3;
    let limit = grip_adjusted(&IdmParams::default(), grip).max_decel;
    let mut w = world(&[straight(0.0, 3000.0)]);
    w.set_weather(grip, f64::INFINITY);
    let lead = w.spawn(LaneId(0), 60.0, 2.0, IdmParams::default()).unwrap();
    let id = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
    let mut hardest = 0.0_f64;
    for _ in 0..300 {
        w.step(0.1);
        if let Some(a) = w.agent(id) {
            hardest = hardest.min(a.accel);
        }
    }
    assert!(w.agent(lead).is_some());
    assert!(hardest >= -limit - 1e-9, "{hardest} vs limit {limit}");
}

#[test]
fn dry_clear_weather_is_the_default() {
    let w = world(&[straight(0.0, 100.0)]);
    assert_eq!(w.weather(), crate::weather::WeatherState::default());
}

/// Steady gap (m) of a follower behind a lead car on a straight road with visibility `vis_m`.
fn steady_gap(vis_m: f64) -> f64 {
    let mut w = world(&[straight(0.0, 4000.0)]);
    w.set_weather(1.0, vis_m);
    let lead = w.spawn(LaneId(0), 120.0, 13.0, IdmParams::default()).unwrap();
    let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
    for _ in 0..1200 {
        w.step(0.1);
    }
    let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
    l.s - l.params.length - f.s
}

#[test]
fn fog_lengthens_the_following_distance() {
    let clear = steady_gap(f64::INFINITY);
    let fog = steady_gap(100.0);
    assert!(fog > 1.2 * clear, "fog {fog} m vs clear {clear} m");
}
