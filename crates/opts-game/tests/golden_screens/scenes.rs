//! The golden scenes: each screen, its steps and both themes, at the wanted sizes.

use super::*;

fn press(app: &mut App, id: &str) {
    let e = find_entity(app.world_mut(), id).unwrap_or_else(|| panic!("no element {id}"));
    app.world_mut().write_message(Activated(e));
    for _ in 0..6 {
        app.update();
    }
}

/// One named scene: a screen and what to do on it before the picture.
pub(super) struct Scene {
    name: &'static str,
    screen: Screen,
    steps: &'static [&'static str],
}

pub(super) const SCENES: &[Scene] = &[
    Scene {
        name: "home",
        screen: Screen::Home,
        steps: &[],
    },
    Scene {
        name: "help",
        screen: Screen::Help,
        steps: &[],
    },
    Scene {
        name: "settings-graphics",
        screen: Screen::Settings,
        steps: &[],
    },
    Scene {
        name: "settings-interface",
        screen: Screen::Settings,
        steps: &["settings/category/interface"],
    },
    Scene {
        name: "plan-1-where",
        screen: Screen::Wizard,
        steps: &[],
    },
    Scene {
        name: "plan-2-what",
        screen: Screen::Wizard,
        steps: &["wizard.stepper.what"],
    },
    Scene {
        name: "plan-4-review",
        screen: Screen::Wizard,
        steps: &["wizard.stepper.review"],
    },
    Scene {
        name: "pause",
        screen: Screen::Pause,
        steps: &[],
    },
    Scene {
        name: "pause-environment",
        screen: Screen::Pause,
        steps: &["pause.environment"],
    },
];

/// The goldens show the screens without an OMSI installation (the home screen's "OMSI 2 was not
/// found" banner): `$OMSI_PATH`, set to run the install-gated tests, would let the screens find
/// one and every home golden would differ. Dropped once, before the first app is built.
fn without_installation() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::env::remove_var("OMSI_PATH"));
}

pub(super) fn render(scene: &Scene, w: u32, h: u32, kind: ThemeKind) -> image::RgbaImage {
    without_installation();
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
    for scene in SCENES
        .iter()
        .filter(|s| only.is_empty() || only.contains(&s.name))
    {
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
    run(
        1920,
        1080,
        &["home", "settings-graphics", "plan-1-where", "pause"],
    );
}
