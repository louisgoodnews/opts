//! The in-game HUD painted over a bright and a dark backdrop, against its goldens and for determinism.

use super::*;

fn render_hud(w: u32, h: u32, backdrop: Color) -> image::RgbaImage {
    use super::hud::{build_hud, Extras};
    let extras = Extras {
        alert: true,
        ..Extras::default()
    };
    let mut app = build_hud(w, h, 1.0, opts_game::hud::VehicleKind::Bus, extras);
    paint(app.world_mut(), w, h, backdrop)
}

#[test]
fn the_hud_matches_its_goldens_over_a_bright_and_a_dark_scene() {
    let mut failures = Vec::new();
    for (tag, backdrop) in [
        ("bright", Color::srgb(0.78, 0.86, 0.95)),
        ("dark", Color::srgb(0.04, 0.05, 0.07)),
    ] {
        let img = render_hud(1280, 720, backdrop);
        if let Err(e) = check(
            &golden_dir(),
            &format!("hud-1280x720-{tag}"),
            &img,
            Tolerance::default(),
        ) {
            failures.push(e);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_hud_painting_is_deterministic_and_the_backdrop_changes_the_picture() {
    let a = render_hud(1280, 720, Color::srgb(0.78, 0.86, 0.95));
    let b = render_hud(1280, 720, Color::srgb(0.78, 0.86, 0.95));
    assert_eq!(a, b);
    let dark = render_hud(1280, 720, Color::srgb(0.04, 0.05, 0.07));
    assert_ne!(a, dark);
}
