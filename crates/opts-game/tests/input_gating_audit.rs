//! Guards the F1-modal input gating (`docs/src/architecture/reporting-feature.md`, A3).
//!
//! 1. Every file that reads keyboard/mouse/UI interaction is on a reviewed list; a new one fails
//!    this test until someone decides whether its systems need `not_modal_active`.
//! 2. The plugin files that register gated systems must still reference `not_modal_active`.

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

/// Files whose systems read input and were reviewed. Gated ones are in `must_reference_gate`.
const REVIEWED: &[&str] = &[
    "opts-bevy/src/camera/input.rs",
    "opts-bevy/src/camera/mode/ego.rs",
    "opts-bevy/src/camera/mode/free_roam.rs",
    "opts-bevy/src/camera/mode/vehicle_views.rs",
    "opts-bevy/src/reporting/trigger.rs",
    "opts-bevy/src/vehicle_sim.rs",
    "opts-bevy/src/vehicle_triggers.rs",
    "opts-game/src/in_game/hud.rs",
    "opts-game/src/in_game/loading_pipeline.rs",
    "opts-game/src/in_game/mod.rs",
    "opts-game/src/in_game/omsi_world/overlay.rs",
    "opts-game/src/pause/mod.rs",
    "opts-game/src/shell/footer.rs",
    "opts-game/src/new_game_screen/picker_systems.rs",
    "opts-game/src/new_game_screen/scroll_systems.rs",
    "opts-game/src/new_game_screen/search_systems.rs",
    "opts-game/src/new_game_screen/setvars_modal.rs",
    "opts-game/src/new_game_screen/systems.rs",
    "opts-game/src/settings_screen/systems.rs",
    // opts-gui widgets: pointer input is blocked by the modal backdrop; keyboard focus is trapped.
    // Tram route choice (Z/X/C) ignores keys while a modal is up; the menu toggle checks the modals itself.
    "opts-game/src/in_game/omsi_world/rail.rs",
    "opts-game/src/in_game/vehicle_menu.rs",
    "opts-gui/src/actions.rs",
    "opts-gui/src/components/button.rs",
    "opts-gui/src/components/checkbox.rs",
    "opts-gui/src/components/combobox.rs",
    "opts-gui/src/components/modal.rs",
    "opts-gui/src/components/notebook.rs",
    "opts-gui/src/components/radio.rs",
    "opts-gui/src/components/switch.rs",
    "opts-gui/src/components/text_area/input.rs",
    "opts-gui/src/components/text_field.rs",
    "opts-gui/src/components/toast.rs",
    "opts-gui/src/components/toplevel.rs",
    "opts-gui/src/components/treeview.rs",
    "opts-gui/src/interaction/activation.rs",
    "opts-gui/src/interaction/focus.rs",
    "opts-gui/src/interaction/layer.rs",
    "opts-gui/src/interaction/mod.rs",
    "opts-gui/src/interaction/state.rs",
];

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
        if !REVIEWED.contains(&rel.as_str()) {
            unknown.push(rel);
        }
    }
    assert!(unknown.is_empty(), "new input-reading files need a gating decision (add `.run_if(not_modal_active)` or list them in REVIEWED): {unknown:?}");
}

#[test]
fn gated_registrations_still_use_the_run_condition() {
    let root = workspace_root().join("crates");
    for (file, min) in [
        ("opts-bevy/src/camera/plugin.rs", 6),
        ("opts-bevy/src/vehicle_sim.rs", 1),
        ("opts-bevy/src/vehicle_triggers.rs", 2),
        ("opts-game/src/shell/mod.rs", 1),
        ("opts-game/src/new_game_screen/mod.rs", 2),
        ("opts-game/src/settings_screen/mod.rs", 1),
        ("opts-game/src/in_game/mod.rs", 1),
    ] {
        let src = std::fs::read_to_string(root.join(file)).expect(file);
        let n = src.matches("not_modal_active").count();
        assert!(
            n >= min,
            "{file}: expected at least {min} uses of not_modal_active, found {n}"
        );
    }
}
