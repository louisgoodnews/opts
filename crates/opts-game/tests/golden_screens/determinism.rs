//! The painter is deterministic, and a changed screen does not pass its golden.

use super::scenes::{render, SCENES};
use super::*;

#[test]
fn the_painter_is_deterministic() {
    let scene = &SCENES[0];
    let a = render(scene, 1280, 720, ThemeKind::Dark);
    let b = render(scene, 1280, 720, ThemeKind::Dark);
    assert_eq!(a, b);
}

#[test]
fn a_changed_screen_does_not_pass_its_golden() {
    // The harness must be able to fail: two different screens never compare equal, and the
    // theme alone changes the picture beyond the tolerance.
    let tol = Tolerance::default();
    let dark = render(&SCENES[0], 1280, 720, ThemeKind::Dark);
    let light = render(&SCENES[0], 1280, 720, ThemeKind::Light);
    let other = render(&SCENES[1], 1280, 720, ThemeKind::Dark);
    for (a, b) in [(&dark, &light), (&dark, &other)] {
        let d = opts_gui::testing::golden::compare(a, b, tol);
        assert!(!opts_gui::testing::golden::passes(&d, tol), "{d:?}");
    }
}
