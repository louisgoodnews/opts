//! Golden screenshots of the screens on **self-authored fixture content** (no OMSI assets or
//! names, ADR-0002), painted by the software painter of `opts_gui::testing::golden` from Bevy's
//! real UI layout and compared with a tolerance.
//!
//! Goldens live in `tests/golden/`. After an intended visual change regenerate them with
//! `UPDATE_GOLDEN=1 cargo test -p opts-game --test golden_screens` and review the diff of the
//! PNGs. A failure writes `<name>.actual.png` and `<name>.diff.png` to `$OPTS_GOLDEN_OUT`
//! (default `tests/golden/_failed/`, git-ignored).
//!
//! Not covered (the painter is not the GPU): images, shadows, the focus ring, gradients. The HUD
//! belongs to its own milestone (M-0064) and the loading screen needs an installation; both are
//! open items of `docs/src/roadmap/phase-9/accessibility-audit.md`.

mod common;

use bevy::prelude::*;
use common::screens::{build, Screen};
use opts_gui::interaction::Activated;
use opts_gui::testing::golden::{check, paint, Tolerance};
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::{AccentPreset, ColorRole, MaterialTheme};
use opts_gui::ui_tree::find_entity;

fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn press(app: &mut App, id: &str) {
    let e = find_entity(app.world_mut(), id).unwrap_or_else(|| panic!("no element {id}"));
    app.world_mut().write_message(Activated(e));
    for _ in 0..6 {
        app.update();
    }
}

/// One named scene: a screen and what to do on it before the picture.
struct Scene {
    name: &'static str,
    screen: Screen,
    steps: &'static [&'static str],
}

const SCENES: &[Scene] = &[
    Scene { name: "home", screen: Screen::Home, steps: &[] },
    Scene { name: "help", screen: Screen::Help, steps: &[] },
    Scene { name: "settings-graphics", screen: Screen::Settings, steps: &[] },
    Scene {
        name: "settings-interface",
        screen: Screen::Settings,
        steps: &["settings/category/interface"],
    },
    Scene { name: "plan-1-where", screen: Screen::Wizard, steps: &[] },
    Scene { name: "plan-2-what", screen: Screen::Wizard, steps: &["wizard.stepper.what"] },
    Scene { name: "plan-4-review", screen: Screen::Wizard, steps: &["wizard.stepper.review"] },
    Scene { name: "pause", screen: Screen::Pause, steps: &[] },
];

fn render(scene: &Scene, w: u32, h: u32, kind: ThemeKind) -> image::RgbaImage {
    let mut app = build(scene.screen, w, h, kind, AccentPreset::default());
    for step in scene.steps {
        press(&mut app, step);
    }
    for _ in 0..4 {
        app.update();
    }
    let bg = MaterialTheme::new(kind, AccentPreset::default())
        .scheme
        .get(ColorRole::Surface);
    paint(app.world_mut(), w, h, bg)
}

fn run(w: u32, h: u32, only: &[&str]) {
    let mut failures = Vec::new();
    for scene in SCENES.iter().filter(|s| only.is_empty() || only.contains(&s.name)) {
        for (kind, tag) in [(ThemeKind::Dark, "dark"), (ThemeKind::Light, "light")] {
            let img = render(scene, w, h, kind);
            let name = format!("{}-{w}x{h}-{tag}", scene.name);
            if let Err(e) = check(&golden_dir(), &name, &img, Tolerance::default()) {
                failures.push(e);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn screens_match_their_goldens_at_1280x720() {
    run(1280, 720, &[]);
}

#[test]
fn key_screens_match_their_goldens_at_1920x1080() {
    run(1920, 1080, &["home", "settings-graphics", "plan-1-where", "pause"]);
}

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
