//! Unit tests for `super` (`inputs.rs`).

use super::*;

#[test]
fn a_vehicle_has_a_positive_lifespan_so_its_failure_limits_are_not_zero() {
    // The scripts take `random(Fehlertoleranz * WearLifespan)` as the distance to a breakdown
    // and fail a drive whose error distance exceeds `Fehlertoleranz * WearLifespan`: with 0
    // the MAN NG313's drive failed at the first metre and the bus could not pull.
    assert!(HostInputs::default().wearlifespan >= 1.0);
}

#[test]
fn the_feed_reaches_the_scripts_and_leaves_script_owned_names_alone() {
    let src = "{frame}\n(L.L.Velocity) (S.L.v)\n(L.S.Time) (S.L.t)\n(L.L.Wheel_RotationSpeed_1_R) (S.L.w)\n(L.L.PAX_Entry2_Req) (S.L.p)\n0.7 (S.L.Throttle)\n3 (S.L.Colorscheme)\n{end}";
    let (mut e, _) = ScriptEngine::from_sources(&[("t.osc", src)]);
    let plan = FeedPlan::new(&mut e);
    assert!(plan.script_owned().contains(&"colorscheme"));
    // The pedals are fed every frame even where a script writes them.
    assert!(!plan.script_owned().contains(&"throttle"));
    assert!(!plan.script_owned().contains(&"brake"));
    let mut h = HostInputs {
        velocity_km_h: 42.5,
        time_of_day_s: 3600.0,
        throttle: 0.2,
        axles: vec![AxleInput::default(); 2],
        ..HostInputs::default()
    };
    h.axles[1].right.speed_rpm = 123.0;
    h.pax_entry_req[2] = true;
    feed(&mut e, &plan, &h);
    e.run_frame(0.016, false);
    assert_eq!(e.get("v"), 42.5);
    assert_eq!(e.get("t"), 3600.0);
    assert_eq!(e.get("w"), 123.0);
    assert_eq!(e.get("p"), 1.0);
    // The script's own `Colorscheme` survives the next feed, its `Throttle` does not.
    feed(&mut e, &plan, &h);
    assert_eq!(e.get("colorscheme"), 3.0);
    assert_eq!(e.get("throttle"), 0.2);
    assert!(plan.zero_fed().contains(&"coll_energy"));
    assert_eq!(plan.status("VELOCITY"), Some(true));
    assert_eq!(plan.status("throttle"), Some(true));
    assert_eq!(plan.status("colorscheme"), Some(false));
    assert_eq!(plan.status("engine_n"), None);
}

#[test]
fn output_rows_match_families() {
    assert_eq!(
        output_row("M_Wheel").map(|r| r.consumer),
        Some(Consumer::Physics)
    );
    assert!(output_row("Axle_Brakeforce_2_L").is_some());
    assert!(output_row("PAX_Exit3_Open").is_some());
    assert!(output_row("PAX_Exit3_Req").is_none());
    assert!(output_row("engine_n").is_none());
}
