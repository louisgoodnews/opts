//! Full-size map tests: following the vehicle and the redraw policy.

use super::*;

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
