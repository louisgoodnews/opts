//! Protocol and speed-radius tests for the mini-map widget.

#[cfg(test)]
use super::*;

#[test]
fn hud_minimap_reads_and_changes_the_settings_and_hud_state_carries_the_map() {
    let mut app = map_app();
    run(&mut app, 30);
    let call = |app: &mut App, params: Value| match protocol::minimap(app.world_mut(), &params) {
        opts_bevy::control::state::Outcome::Done(r) => r,
        _ => panic!("not immediate"),
    };
    let got = call(&mut app, json!({})).expect("get");
    assert_eq!(got["enabled"], true);
    assert_eq!(got["zoom_m"], 200.0);
    assert_eq!(got["zoom_mode"], "speed");
    assert!(
        got["radius_m"].is_number() && got["target_m"].is_number() && got["speed_kmh"].is_number()
    );
    let fixed = call(&mut app, json!({ "zoom_mode": "fixed" })).expect("zoom_mode");
    assert_eq!(fixed["zoom_mode"], "fixed");
    assert!(
        call(&mut app, json!({ "zoom_mode": "x" })).is_err(),
        "unknown mode is rejected"
    );
    call(&mut app, json!({ "zoom_mode": "speed" })).expect("back");
    assert_eq!(got["orientation"], "heading");
    assert_eq!(got["zoom_steps_m"], json!([100.0, 200.0, 500.0, 1000.0]));
    assert!(got["redraws"].as_u64().expect("n") > 0);
    assert_eq!(got["markers"], 2);
    let set = call(
        &mut app,
        json!({"zoom": 1000, "orientation": "north", "enabled": true}),
    )
    .expect("set");
    assert_eq!(set["zoom_m"], 1000.0);
    assert_eq!(set["orientation"], "north");
    for bad in [
        json!({"zoom": 300}),
        json!({"orientation": "sideways"}),
        json!({"enabled": "yes"}),
    ] {
        let e = call(&mut app, bad.clone()).expect_err("refused");
        assert_eq!(e.code, opts_cli::protocol::code::INVALID_PARAMS, "{bad}");
    }
    assert_eq!(
        call(&mut app, json!({})).expect("get")["zoom_m"],
        1000.0,
        "a refused call changes nothing"
    );
    let off = call(&mut app, json!({"enabled": false})).expect("off");
    assert_eq!(off["enabled"], false);
    run(&mut app, 3);
    let state = match protocol::state_for_tests(app.world_mut()) {
        opts_bevy::control::state::Outcome::Done(r) => r.expect("state"),
        _ => panic!("not immediate"),
    };
    assert_eq!(state["minimap"]["shown"], false);
    assert_eq!(state["minimap"]["orientation"], "north");
}

#[test]
fn the_mini_map_radius_follows_the_speed_and_the_fixed_mode_ignores_it() {
    let mut app = map_app();
    let state = |app: &App| app.world().resource::<minimap::MiniMapState>().clone();
    // `sample_values` reads the road speed of the player's controller every frame.
    let set_speed = |app: &mut App, kmh: f32| {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut opts_bevy::VehicleController, With<opts_bevy::PlayerVehicle>>();
        for mut c in q.iter_mut(app.world_mut()) {
            c.sim.chassis.local_velocity.y = kmh / 3.6;
        }
    };
    // Stopped: the near radius (a third of the 200 m step).
    set_speed(&mut app, 0.0);
    run(&mut app, 120);
    let s = state(&app);
    assert_eq!(s.config.zoom_mode, minimap::ZoomMode::Speed);
    assert!(
        (s.radius_m - 200.0 / 3.0).abs() < 2.0,
        "stopped: {}",
        s.radius_m
    );
    // Cruise: the radius grows to the step and stays there.
    set_speed(&mut app, 60.0);
    run(&mut app, 900);
    let s = state(&app);
    assert!(
        (s.radius_m / 200.0 - 1.0).abs() < 0.05,
        "cruise: {}",
        s.radius_m
    );
    assert!(s.speed_kmh > 55.0, "filtered speed {}", s.speed_kmh);
    // Stopping again: back to the near radius.
    set_speed(&mut app, 0.0);
    run(&mut app, 900);
    assert!((state(&app).radius_m / (200.0 / 3.0) - 1.0).abs() < 0.05);
    // Fixed mode: the speed does not matter.
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw(minimap::SETTING_ZOOM_MODE, "fixed");
    set_speed(&mut app, 60.0);
    run(&mut app, 60);
    assert_eq!(state(&app).radius_m, 200.0);
}
