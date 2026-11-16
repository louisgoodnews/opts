//! The degree of the setup: which state a drive stops at, for each degree.

use super::*;

#[test]
fn degree_zero_starts_nothing() {
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    assert!(
        drive(&mut e, &mut run, &at(0.0), 10.0),
        "nothing to do is settled"
    );
    assert_eq!(e.get("elec_busbar_main"), 0.0);
    assert_eq!(e.get("engine_on"), 0.0);
}

#[test]
fn each_degree_stops_at_its_state() {
    for (degree, electrics, engine_on) in [
        (0.25, 1.0, 0.0),
        (0.5, 1.0, 0.0),
        (0.75, 1.0, 1.0),
        (1.0, 1.0, 1.0),
    ] {
        let mut e = engine(MAN_LIKE);
        let mut run = SetupRun::default();
        assert!(
            drive(&mut e, &mut run, &at(degree), 12.0),
            "degree {degree} settles"
        );
        assert_eq!(
            e.get("elec_busbar_main"),
            electrics,
            "electrics at {degree}"
        );
        assert_eq!(e.get("engine_on"), engine_on, "engine at {degree}");
    }
}

#[test]
fn raising_the_degree_later_continues_from_where_it_stopped() {
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    drive(&mut e, &mut run, &at(0.25), 6.0);
    assert_eq!(e.get("engine_on"), 0.0);
    // Time continues: the run is driven with a larger `now` than before.
    let mut now = 6.0;
    let want = at(0.75);
    while now < 20.0 {
        now += 1.0 / 60.0;
        e.run_frame(1.0 / 60.0, false);
        run.advance(&mut e, &want, now);
    }
    assert_eq!(e.get("engine_on"), 1.0);
}
