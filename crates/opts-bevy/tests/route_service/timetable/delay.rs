//! The delay the timetable reports: the clock minus the arrival, wrapped at midnight.

use super::*;

#[test]
fn the_delay_is_the_clock_minus_the_arrival_at_the_current_stop() {
    let route = handle();
    route.select_run("33", 0, 0).expect("run");
    let mut h = vehicle_callbacks_with_context(VehicleHostContext {
        route: route.clone(),
        ..Default::default()
    });
    route.set_clock(28_800.0 + 30.0);
    assert_eq!(
        call(&mut h, "getttdelay", &[]).numbers,
        [30.0],
        "30 s late at the first stop"
    );
    route.set_stop(1);
    route.set_clock(28_800.0 + 200.0);
    assert_eq!(
        call(&mut h, "getttdelay", &[]).numbers,
        [-40.0],
        "40 s early at stop 1"
    );
    route.clear_schedule();
    route.set_clock(40_000.0);
    assert_eq!(
        call(&mut h, "getttdelay", &[]).numbers,
        [0.0],
        "free driving has none"
    );
}

#[test]
fn the_delay_wraps_at_midnight() {
    let route = handle();
    route.select_run("33", 0, 0).expect("run");
    // The run's first stop is at 08:00; a clock at 08:00 the next day is on time, 23:50 the
    // evening before is 8 h 10 min early (not 15 h 50 min late).
    route.set_clock(8.0 * 3600.0);
    assert_eq!(route.with(|s| s.delay_s), 0.0);
    route.set_clock(23.0 * 3600.0 + 50.0 * 60.0);
    assert_eq!(
        route.with(|s| s.delay_s),
        15.0 * 3600.0 + 50.0 * 60.0 - 86_400.0
    );
}
