//! Guards the F1-modal input gating (`docs/src/architecture/reporting-feature.md`, A3).
//!
//! 1. Every file that reads keyboard/mouse/UI interaction is on a reviewed list (split per crate
//!    into `reviewed_*.rs`); a new one fails until someone decides whether its systems need
//!    `not_modal_active`.
//! 2. Every reviewed path still exists, so the lists cannot go stale after a split.
//! 3. The plugin files that register gated systems must still reference `not_modal_active`.

mod reviewed_bevy;
mod reviewed_game;
mod reviewed_gui;

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn reads_input(src: &str) -> bool {
    [
        "ButtonInput<",
        "MessageReader<MouseWheel>",
        "MessageReader<MouseMotion>",
        "MessageReader<KeyboardInput>",
        "Changed<Interaction>",
    ]
    .iter()
    .any(|needle| src.contains(needle))
}

/// Every reviewed path, relative to `crates/`.
fn reviewed() -> impl Iterator<Item = &'static str> {
    reviewed_bevy::REVIEWED
        .iter()
        .chain(reviewed_game::REVIEWED)
        .chain(reviewed_gui::REVIEWED)
        .copied()
}

#[test]
fn every_input_reading_file_has_been_reviewed() {
    let root = workspace_root().join("crates");
    let mut files = Vec::new();
    for krate in ["opts-bevy", "opts-game", "opts-gui"] {
        rust_files(&root.join(krate).join("src"), &mut files);
    }
    let mut unknown = Vec::new();
    for f in files {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        if !reads_input(&src) {
            continue;
        }
        let rel = f
            .strip_prefix(&root)
            .expect("under crates")
            .to_string_lossy()
            .replace('\\', "/");
        if !reviewed().any(|r| r == rel) {
            unknown.push(rel);
        }
    }
    assert!(unknown.is_empty(), "new input-reading files need a gating decision (add `.run_if(not_modal_active)` or list them in the reviewed lists): {unknown:?}");
}

#[test]
fn every_reviewed_path_exists() {
    let root = workspace_root().join("crates");
    let missing: Vec<&str> = reviewed().filter(|p| !root.join(p).is_file()).collect();
    assert!(missing.is_empty(), "reviewed lists name files that no longer exist (update them after a split): {missing:?}");
}

#[test]
fn gated_registrations_still_use_the_run_condition() {
    let root = workspace_root().join("crates");
    for (file, min) in [
        ("opts-bevy/src/camera/plugin.rs", 7),
        ("opts-bevy/src/vehicle_sim/plugin.rs", 1),
        ("opts-bevy/src/vehicle_triggers.rs", 2),
        ("opts-game/src/shell/mod.rs", 1),
        ("opts-game/src/new_game_screen/mod.rs", 2),
        ("opts-game/src/settings_screen/mod.rs", 1),
        ("opts-game/src/in_game/late.rs", 1),
        ("opts-game/src/session_vehicles/mod.rs", 1),
        ("opts-game/src/hud/minimap/mod.rs", 1),
        ("opts-game/src/save/quick.rs", 1),
    ] {
        let src = std::fs::read_to_string(root.join(file)).expect(file);
        let n = src.matches("not_modal_active").count();
        assert!(
            n >= min,
            "{file}: expected at least {min} uses of not_modal_active, found {n}"
        );
    }
}
