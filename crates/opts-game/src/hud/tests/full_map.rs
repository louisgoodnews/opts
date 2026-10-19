//! The full-size map (M-0108): opening, following, closing, the redraw policy and the input hold.

use super::map_widget::{map_app, run};
use super::*;
use crate::hud::fullmap::{FullMapRequest, FullMapRoot, FullMapState};

fn full(app: &App) -> FullMapState {
    app.world().resource::<FullMapState>().clone()
}

fn press(app: &mut App, keys: &[KeyCode]) {
    use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
    use bevy::input::ButtonState;
    for state in [ButtonState::Pressed, ButtonState::Released] {
        for k in keys {
            app.world_mut().write_message(KeyboardInput {
                key_code: *k,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        }
        for _ in 0..2 {
            app.update();
        }
    }
}

fn open_map(app: &mut App) {
    app.world_mut().resource_mut::<FullMapState>().request = Some(FullMapRequest::Open);
    run(app, 3);
}

fn roots(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<FullMapRoot>>()
        .iter(app.world())
        .count()
}

#[test]
fn the_map_opens_centred_on_the_vehicle_and_draws_roads_and_markers() {
    let mut app = map_app();
    run(&mut app, 2);
    assert!(!full(&app).open, "closed until asked");
    assert_eq!(roots(&mut app), 0);
    open_map(&mut app);
    let s = full(&app);
    assert!(s.open && s.follow && s.holds_input());
    // The vehicle stands at 30 m east, 20 m north.
    assert!((s.viewport.centre.x - 30.0).abs() < 0.01 && (s.viewport.centre.y - 20.0).abs() < 0.01);
    assert_eq!(roots(&mut app), 1, "one overlay");
    assert!(s.frames >= 1 && s.lines > 0, "roads of the fixture: {s:?}");
    assert_eq!(s.markers, 2, "the player and the other session vehicle");
    assert_eq!(
        s.texture_px, 1024,
        "the fallback screen is 1280 x 720, the texture is capped"
    );
}

#[test]
fn f6_and_esc_close_it_and_the_input_stays_held_for_two_frames() {
    let mut app = map_app();
    open_map(&mut app);
    press(&mut app, &[KeyCode::F6]);
    let s = full(&app);
    assert!(!s.open, "F6 closes");
    run(&mut app, 3);
    assert_eq!(roots(&mut app), 0, "the overlay is gone");
    assert!(!full(&app).holds_input(), "and the hold is released again");
    // Esc closes too, and F6 opens again.
    press(&mut app, &[KeyCode::F6]);
    assert!(full(&app).open, "F6 opens");
    press(&mut app, &[KeyCode::Escape]);
    assert!(!full(&app).open, "Esc closes");
}

#[test]
fn the_hold_keeps_the_press_that_closed_the_map_from_opening_anything_else() {
    let mut app = map_app();
    open_map(&mut app);
    app.world_mut().resource_mut::<FullMapState>().request = Some(FullMapRequest::Close);
    run(&mut app, 1);
    let s = full(&app);
    assert!(
        !s.open && s.holds_input(),
        "closed, but the keys are still held back: {s:?}"
    );
    run(&mut app, 3);
    assert!(!full(&app).holds_input());
}

#[test]
fn another_dialog_that_holds_the_input_keeps_the_map_shut() {
    let mut app = map_app();
    app.insert_resource(opts_bevy::reporting::ModalActive(true));
    open_map(&mut app);
    assert!(!full(&app).open, "the report dialog is up");
    app.insert_resource(opts_bevy::reporting::ModalActive(false));
    app.insert_resource(opts_bevy::reporting::MenuModalActive(true));
    open_map(&mut app);
    assert!(!full(&app).open, "the vehicle menu is up");
    app.insert_resource(opts_bevy::reporting::MenuModalActive(false));
    open_map(&mut app);
    assert!(full(&app).open);
}

#[test]
fn following_tracks_the_vehicle_until_the_view_is_moved_and_home_follows_again() {
    let mut app = map_app();
    open_map(&mut app);
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>()
        .iter(app.world())
        .next()
        .expect("player");
    let move_to = |app: &mut App, x: f32| {
        app.world_mut()
            .entity_mut(player)
            .insert(GlobalTransform::from(Transform::from_xyz(x, 0.0, -20.0)));
    };
    move_to(&mut app, 90.0);
    run(&mut app, 2);
    assert!(
        (full(&app).viewport.centre.x - 90.0).abs() < 0.01,
        "the map follows"
    );
    // A view set by hand stays where it is.
    {
        let mut s = app.world_mut().resource_mut::<FullMapState>();
        s.viewport.centre = bevy::math::DVec2::new(500.0, 500.0);
        s.follow = false;
    }
    move_to(&mut app, 120.0);
    run(&mut app, 2);
    assert!(
        (full(&app).viewport.centre.x - 500.0).abs() < 0.01,
        "it stays"
    );
    press(&mut app, &[KeyCode::Home]);
    run(&mut app, 2);
    let s = full(&app);
    assert!(
        s.follow && (s.viewport.centre.x - 120.0).abs() < 0.01,
        "Home follows again: {s:?}"
    );
}

#[test]
fn a_still_map_redraws_twice_a_second_and_a_moving_vehicle_ten_times() {
    // 16 ms per frame: 125 frames are two seconds.
    let mut app = map_app();
    open_map(&mut app);
    let before = full(&app).frames;
    run(&mut app, 125);
    let still = full(&app).frames - before;
    assert!(
        (3..=6).contains(&still),
        "{still} redraws in 2 s while nothing moves"
    );

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>()
        .iter(app.world())
        .next()
        .expect("player");
    let before = full(&app).frames;
    for i in 0..125 {
        app.world_mut()
            .entity_mut(player)
            .insert(GlobalTransform::from(Transform::from_xyz(
                30.0 + i as f32 * 0.5,
                0.0,
                -20.0,
            )));
        app.update();
    }
    let moving = full(&app).frames - before;
    assert!(
        (15..=24).contains(&moving),
        "{moving} redraws in 2 s while it drives"
    );
}

#[test]
fn a_change_of_the_view_redraws_in_the_next_frame_and_stays_in_budget() {
    let mut app = map_app();
    open_map(&mut app);
    run(&mut app, 3);
    let before = full(&app).frames;
    {
        let mut s = app.world_mut().resource_mut::<FullMapState>();
        s.viewport.span_m = 300.0;
        s.touch();
    }
    run(&mut app, 1);
    let s = full(&app);
    assert_eq!(s.frames, before + 1, "one frame after the change");
    // A sanity bound only: a dev build under the load of the whole test suite is many times slower
    // than the release build, where the redraw is measured live (2 to 4 ms, see M-0108).
    assert!(s.last_us < 60_000, "{} us", s.last_us);
}

#[test]
fn the_protocol_opens_and_moves_the_map_and_refuses_bad_values() {
    use serde_json::json;
    let mut app = map_app();
    let call = |app: &mut App, params: serde_json::Value| match protocol::fullmap(
        app.world_mut(),
        &params,
    ) {
        opts_bevy::control::state::Outcome::Done(r) => r,
        _ => panic!("not immediate"),
    };
    let v = call(
        &mut app,
        json!({"open": true, "span": 2500.0, "x": 10.0, "y": -40.0}),
    )
    .expect("ok");
    assert_eq!(v["span_m"], 2500.0);
    assert_eq!(v["follow"], false);
    run(&mut app, 3);
    let s = full(&app);
    assert!(s.open, "the request opened it");
    // The view that was set is kept; it is not re-centred on the vehicle.
    assert_eq!(s.viewport.span_m, 2500.0);
    assert!((s.viewport.centre.x - 10.0).abs() < 1e-9 && (s.viewport.centre.y + 40.0).abs() < 1e-9);
    let read = call(&mut app, json!({})).expect("read");
    assert_eq!(read["open"], true);
    assert!(read["redraws"].as_u64().expect("n") > 0 && read["min_span_m"] == 50.0);
    for bad in [
        json!({"span": 10.0}),
        json!({"span": 50000.0}),
        json!({"x": 1.0}),
    ] {
        assert!(call(&mut app, bad.clone()).is_err(), "{bad}");
    }
    let closed = call(&mut app, json!({"open": false})).expect("close");
    assert!(closed["open"].is_boolean());
    run(&mut app, 2);
    assert!(!full(&app).open);
}
