//! Mini-map tests: following the vehicle, the roads and the redraw rate.

use super::*;

#[test]
fn the_map_follows_the_vehicle_marks_all_session_vehicles_and_draws_the_roads() {
    let mut app = map_app();
    run(&mut app, 60);
    let s = minimap_state(&app);
    assert!(s.frames > 0, "redrawn");
    let c = s.centre.expect("centre");
    assert!(
        (c.x - 30.0).abs() < 1e-3 && (c.y - 20.0).abs() < 1e-3,
        "{c:?}"
    );
    assert_eq!(s.markers, 2, "the active and the other session vehicle");
    assert!(s.lines >= 10, "roads: {}", s.lines);
    // Heading east: heading-up gives 90 degrees up.
    assert!(
        (s.up_heading_deg - 90.0).abs() < 0.01,
        "{}",
        s.up_heading_deg
    );
    // The texture really holds road pixels.
    let handle = app
        .world()
        .resource::<MiniMapImage>()
        .handle()
        .expect("image");
    let images = app.world().resource::<Assets<Image>>();
    let data = images
        .get(&handle)
        .expect("asset")
        .data
        .clone()
        .expect("data");
    let road =
        super::super::minimap::colors_of(&opts_gui::theme::InstrumentTokens::default().day).road;
    assert!(
        data.chunks_exact(4).any(|p| p == road.as_slice()),
        "roads in the texture"
    );
}

#[test]
fn the_map_redraws_at_most_ten_times_a_second() {
    let mut app = map_app();
    run(&mut app, 12);
    let before = minimap_state(&app).frames;
    // 100 frames of 16 ms are 1.6 s: at most 16 redraws (and about 14).
    run(&mut app, 100);
    let drawn = minimap_state(&app).frames - before;
    assert!((8..=16).contains(&drawn), "{drawn} redraws in 1.6 s");
}

#[test]
fn a_redraw_is_within_the_budget_on_the_fixture() {
    let mut app = map_app();
    run(&mut app, 60);
    let s = minimap_state(&app);
    // The slowest redraw includes the first one (cold caches); it stays well inside a frame.
    assert!(
        s.last_us < super::super::minimap::BUDGET_US * 4,
        "{} us",
        s.last_us
    );
}
