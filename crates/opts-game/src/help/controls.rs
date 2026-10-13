//! The controls table, generated from [`GROUPS`], the one registry of the keyboard bindings that
//! Help prints. A test checks every listed key against the input systems' source files.

use bevy::prelude::*;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use crate::shell::keys;

/// One binding: what is shown, the keys behind it, what it does and where it is handled.
// `keys` and `source` document the binding and are checked by the tests below.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    /// The keys as printed (key names are not translated).
    pub shown: &'static str,
    /// Every key the binding uses; [`tests`] check each is read by `source`.
    pub keys: &'static [KeyCode],
    /// Message key of what the keys do.
    pub action: &'static str,
    /// The source file (repository path) whose input system reads the keys.
    pub source: &'static str,
}

/// A group: its message key and bindings.
pub type Group = (&'static str, &'static [Binding]);

/// Shorthand for [`Binding`] rows.
const fn bind(
    shown: &'static str,
    keys: &'static [KeyCode],
    action: &'static str,
    source: &'static str,
) -> Binding {
    Binding {
        shown,
        keys,
        action,
        source,
    }
}

const SIM: &str = "crates/opts-bevy/src/vehicle_sim.rs";
const RAIL: &str = "crates/opts-game/src/in_game/omsi_world/rail.rs";
const CAMERA: &str = "crates/opts-bevy/src/camera/input.rs";
const VIEWPOINT: &str = "crates/opts-bevy/src/camera/viewpoint.rs";
const EGO: &str = "crates/opts-bevy/src/camera/mode/ego.rs";
const PAUSE: &str = "crates/opts-gui/src/actions.rs";
const SIM_PAUSE: &str = "crates/opts-game/src/in_game/mod.rs";
const HELP: &str = "crates/opts-game/src/help/mod.rs";
const THEME: &str = "crates/opts-game/src/shell/footer.rs";
const DEV: &str = "crates/opts-game/src/in_game/omsi_world/overlay.rs";

/// The single registry of the keyboard bindings that the Help page prints. The input systems keep
/// reading keys directly (they are not driven by this table); the test in this module reads their
/// source files and fails when a listed key is no longer used there. Add a row here when an input
/// system gets a new key, and name the row's file in the system's doc comment.
pub const GROUPS: &[Group] = &[
    (
        keys::HELP_GROUP_DRIVING,
        &[
            bind(
                "W / S",
                &[KeyCode::KeyW, KeyCode::KeyS],
                keys::HELP_KEY_THROTTLE,
                SIM,
            ),
            bind(
                "← / →",
                &[KeyCode::ArrowLeft, KeyCode::ArrowRight],
                keys::HELP_KEY_STEER,
                SIM,
            ),
            bind(".", &[KeyCode::Period], keys::HELP_KEY_PARKING, SIM),
            bind(
                "D / N / R",
                &[KeyCode::KeyD, KeyCode::KeyN, KeyCode::KeyR],
                keys::HELP_KEY_GEARS,
                SIM,
            ),
            bind(
                "Z / X / C",
                &[KeyCode::KeyZ, KeyCode::KeyX, KeyCode::KeyC],
                keys::HELP_KEY_TRAM,
                RAIL,
            ),
        ],
    ),
    (
        keys::HELP_GROUP_CAMERA,
        &[
            bind(
                "1 – 5",
                &[
                    KeyCode::Digit1,
                    KeyCode::Digit2,
                    KeyCode::Digit3,
                    KeyCode::Digit4,
                    KeyCode::Digit5,
                ],
                keys::HELP_KEY_CAMERA,
                CAMERA,
            ),
            bind("0", &[KeyCode::Digit0], keys::HELP_KEY_CAMERA_PREV, CAMERA),
            bind(
                "Shift + Space",
                &[KeyCode::ShiftLeft, KeyCode::Space],
                keys::HELP_KEY_CAMERA_RESET,
                CAMERA,
            ),
            bind(
                "Tab",
                &[KeyCode::Tab],
                keys::HELP_KEY_VIEWPOINT,
                VIEWPOINT,
            ),
        ],
    ),
    (
        keys::HELP_GROUP_MENUS,
        &[
            bind("Esc", &[KeyCode::Escape], keys::HELP_KEY_PAUSE_MENU, PAUSE),
            bind("P", &[KeyCode::KeyP], keys::HELP_KEY_SIM_PAUSE, SIM_PAUSE),
            bind("Alt", &[KeyCode::AltLeft], keys::HELP_KEY_VEHICLE_MENU, EGO),
            bind("F1", &[KeyCode::F1], keys::HELP_KEY_HELP, HELP),
            bind("F2", &[KeyCode::F2], keys::HELP_KEY_THEME, THEME),
            bind("F10", &[KeyCode::F10], keys::HELP_KEY_DEV, DEV),
        ],
    ),
];

/// Spawns the table into `parent`.
pub fn spawn_controls(parent: &mut ChildSpawnerCommands) {
    for (group, rows) in GROUPS {
        parent.spawn((
            Text::new(*group),
            L10nKey(group),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleMedium),
            ForegroundRole(ColorRole::OnSurface),
        ));
        for Binding {
            shown: keys_text,
            action,
            ..
        } in *rows
        {
            parent
                .spawn(Node {
                    column_gap: Val::Px(16.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            min_width: Val::Px(120.0),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundRole(ColorRole::SurfaceContainerHighest),
                        RadiusToken(RadiusLevel::Small),
                    ))
                    .with_children(|chip| {
                        chip.spawn((
                            Text::new(*keys_text),
                            TextColor::default(),
                            TypeStyleToken(TypeScale::Label),
                            ForegroundRole(ColorRole::OnSurface),
                        ));
                    });
                    row.spawn((
                        Text::new(*action),
                        L10nKey(action),
                        TextColor::default(),
                        TypeStyleToken(TypeScale::BodyMedium),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn every_listed_key_is_read_by_its_input_system() {
        for (_, rows) in GROUPS {
            for b in *rows {
                let text = std::fs::read_to_string(root().join(b.source))
                    .unwrap_or_else(|e| panic!("{}: {e}", b.source));
                for key in b.keys {
                    assert!(
                        text.contains(&format!("KeyCode::{key:?}")),
                        "{} lists {key:?} but {} never reads it",
                        b.shown,
                        b.source
                    );
                }
            }
        }
    }

    #[test]
    fn every_row_has_keys_and_a_shown_text_and_no_key_is_listed_twice() {
        let mut seen = std::collections::HashSet::new();
        for (_, rows) in GROUPS {
            for b in *rows {
                assert!(!b.keys.is_empty() && !b.shown.is_empty(), "{b:?}");
                for key in b.keys {
                    assert!(seen.insert(*key), "{key:?} is listed twice");
                }
            }
        }
    }
}
