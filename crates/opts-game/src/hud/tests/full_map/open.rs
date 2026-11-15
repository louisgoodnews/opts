//! Full-size map tests: opening and closing, and the input hold of the keys.

use super::*;

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
