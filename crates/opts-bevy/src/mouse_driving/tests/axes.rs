//! Pedal axes from the cursor position in the window.

use super::*;

#[test]
fn the_middle_is_neither_pedal_and_straight() {
    assert_eq!(axes_of(Vec2::new(500.0, 300.0), SIZE), (0.0, 0.0, 0.0));
    // Inside the dead zone nothing moves.
    let (t, b, s) = axes_of(Vec2::new(510.0, 305.0), SIZE);
    assert_eq!((t, b, s), (0.0, 0.0, 0.0));
}

#[test]
fn height_is_throttle_above_the_middle_and_brake_below() {
    let (t, b, _) = axes_of(Vec2::new(500.0, 0.0), SIZE);
    assert_eq!((t, b), (1.0, 0.0), "top edge: full throttle");
    let (t, b, _) = axes_of(Vec2::new(500.0, 600.0), SIZE);
    assert_eq!((t, b), (0.0, 1.0), "bottom edge: full brake");
    let (t, _, _) = axes_of(Vec2::new(500.0, 150.0), SIZE);
    assert!(
        (t - 0.5 / 0.96 + 0.04 / 0.96).abs() < 0.2 && (0.4..0.6).contains(&t),
        "a quarter up: {t}"
    );
}

#[test]
fn width_is_steering_with_full_lock_at_the_edges() {
    let (_, _, s) = axes_of(Vec2::new(0.0, 300.0), SIZE);
    assert_eq!(s, -1.0);
    let (_, _, s) = axes_of(Vec2::new(1000.0, 300.0), SIZE);
    assert_eq!(s, 1.0);
    let (_, _, s) = axes_of(Vec2::new(750.0, 300.0), SIZE);
    assert!((0.4..0.6).contains(&s), "{s}");
}

#[test]
fn a_cursor_outside_the_window_is_clamped() {
    let (t, _, s) = axes_of(Vec2::new(-400.0, -300.0), SIZE);
    assert_eq!((t, s), (1.0, -1.0));
}
