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
    /// The rebindable actions behind the keys (`opts_bevy::action_keys`): the chip shows their
    /// current chords, `shown` is the default; empty for keys that cannot move.
    pub ids: &'static [&'static str],
}

/// The chip of a rebindable row: its text follows the player's bindings.
#[derive(Component, Debug, Clone, Copy)]
pub struct ActionChip(pub &'static [&'static str]);

/// Writes the current chords of the rebindable rows into their chips.
pub fn sync_action_chips(
    keys: Option<Res<opts_bevy::ActionKeys>>,
    mut chips: Query<(&ActionChip, &mut Text)>,
    added: Query<(), Added<ActionChip>>,
) {
    let changed = keys.as_ref().is_some_and(|k| k.is_changed()) || !added.is_empty();
    if !changed {
        return;
    }
    let keys = opts_bevy::action_keys_of(&keys);
    for (chip, mut text) in &mut chips {
        let shown: Vec<String> = chip.0.iter().map(|id| keys.label(id)).collect();
        let shown = shown.join(" / ");
        if text.0 != shown {
            text.0 = shown;
        }
    }
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
        ids: &[],
    }
}

/// A [`Binding`] whose keys the player can change.
const fn rebindable(
    ids: &'static [&'static str],
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
        ids,
    }
}

const SIM: &str = "crates/opts-bevy/src/vehicle_sim.rs";
const RAIL: &str = "crates/opts-game/src/in_game/omsi_world/rail.rs";
const RAIL_CONTROLS: &str = "crates/opts-bevy/src/rail_controls.rs";
const CAMERA: &str = "crates/opts-bevy/src/camera/input.rs";
const VIEWPOINT: &str = "crates/opts-bevy/src/camera/viewpoint.rs";
const EGO: &str = "crates/opts-bevy/src/camera/mode/ego.rs";
const PAUSE: &str = "crates/opts-gui/src/actions.rs";
const SIM_PAUSE: &str = "crates/opts-game/src/in_game/mod.rs";
const HELP: &str = "crates/opts-game/src/help/mod.rs";
const THEME: &str = "crates/opts-game/src/shell/footer.rs";
const DEV: &str = "crates/opts-game/src/in_game/omsi_world/overlay.rs";
const MINIMAP: &str = "crates/opts-game/src/hud/minimap/mod.rs";
const FULLMAP: &str = "crates/opts-game/src/hud/fullmap/mod.rs";
const DOOR_CAMERA: &str = "crates/opts-bevy/src/door_camera.rs";
const SESSION: &str = "crates/opts-game/src/session_vehicles/mod.rs";
const QUICK_SAVE: &str = "crates/opts-game/src/save/quick.rs";

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
            rebindable(
                &["parking_brake"],
                ".",
                &[KeyCode::Period],
                keys::HELP_KEY_PARKING,
                SIM,
            ),
            bind(
                "D / N / R",
                &[KeyCode::KeyD, KeyCode::KeyN, KeyCode::KeyR],
                keys::HELP_KEY_GEARS,
                SIM,
            ),
        ],
    ),
    (
        keys::HELP_GROUP_RAIL,
        &[
            rebindable(
                &["rail_forward", "rail_reverse"],
                "W / S",
                &[KeyCode::KeyW, KeyCode::KeyS],
                keys::HELP_KEY_RAIL_DIRECTION,
                RAIL_CONTROLS,
            ),
            rebindable(
                &["rail_lever_up", "rail_lever_down"],
                "Q / Y",
                &[KeyCode::KeyQ, KeyCode::KeyZ],
                keys::HELP_KEY_RAIL_LEVER,
                RAIL_CONTROLS,
            ),
            rebindable(
                &[
                    "rail_branch_left",
                    "rail_branch_straight",
                    "rail_branch_right",
                ],
                "← ↑ →",
                &[KeyCode::ArrowLeft, KeyCode::ArrowUp, KeyCode::ArrowRight],
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
            bind("Tab", &[KeyCode::Tab], keys::HELP_KEY_VIEWPOINT, VIEWPOINT),
        ],
    ),
    (
        keys::HELP_GROUP_MAP,
        &[
            rebindable(
                &["minimap_toggle"],
                "F7",
                &[KeyCode::F7],
                keys::HELP_KEY_MINIMAP,
                MINIMAP,
            ),
            rebindable(
                &["minimap_zoom_in", "minimap_zoom_out"],
                "+ / -",
                &[KeyCode::Equal, KeyCode::Minus],
                keys::HELP_KEY_MINIMAP_ZOOM,
                MINIMAP,
            ),
            rebindable(
                &["full_map"],
                "F6",
                &[KeyCode::F6],
                keys::HELP_KEY_FULLMAP,
                FULLMAP,
            ),
            rebindable(
                &["door_camera"],
                "F4",
                &[KeyCode::F4],
                keys::HELP_KEY_DOOR_CAMERA,
                DOOR_CAMERA,
            ),
        ],
    ),
    (
        keys::HELP_GROUP_MENUS,
        &[
            bind("Esc", &[KeyCode::Escape], keys::HELP_KEY_PAUSE_MENU, PAUSE),
            bind("P", &[KeyCode::KeyP], keys::HELP_KEY_SIM_PAUSE, SIM_PAUSE),
            bind("Alt", &[KeyCode::AltLeft], keys::HELP_KEY_VEHICLE_MENU, EGO),
            rebindable(
                &["switch_vehicle"],
                "F8",
                &[KeyCode::F8],
                keys::HELP_KEY_SWITCH_VEHICLE,
                SESSION,
            ),
            rebindable(
                &["quick_save"],
                "F5",
                &[KeyCode::F5],
                keys::HELP_KEY_QUICK_SAVE,
                QUICK_SAVE,
            ),
            rebindable(
                &["quick_load"],
                "F9",
                &[KeyCode::F9],
                keys::HELP_KEY_QUICK_LOAD,
                QUICK_SAVE,
            ),
            rebindable(&["help"], "F1", &[KeyCode::F1], keys::HELP_KEY_HELP, HELP),
            rebindable(
                &["theme"],
                "F2",
                &[KeyCode::F2],
                keys::HELP_KEY_THEME,
                THEME,
            ),
            rebindable(
                &["developer_panel"],
                "F10",
                &[KeyCode::F10],
                keys::HELP_KEY_DEV,
                DEV,
            ),
        ],
    ),
];

/// Spawns the table into `parent`.
pub fn spawn_controls(parent: &mut ChildSpawnerCommands) {
    // The vehicle system keys are rebindable (Settings, Controls), so the table does not list a
    // fixed key for them; this note points to the defaults and the page.
    parent.spawn((
        Text::new(keys::HELP_GROUP_VEHICLE_KEYS),
        L10nKey(keys::HELP_GROUP_VEHICLE_KEYS),
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
    parent.spawn((
        Text::new(keys::HELP_VEHICLE_KEYS_NOTE),
        L10nKey(keys::HELP_VEHICLE_KEYS_NOTE),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));
    super::vehicle_controls::spawn_list(parent);
    parent.spawn((
        Text::new(keys::HELP_GROUP_MOUSE_DRIVING),
        L10nKey(keys::HELP_GROUP_MOUSE_DRIVING),
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
    parent.spawn((
        Text::new(keys::HELP_MOUSE_DRIVING_NOTE),
        L10nKey(keys::HELP_MOUSE_DRIVING_NOTE),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));
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
            ids,
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
                        let mut text = chip.spawn((
                            Text::new(*keys_text),
                            TextColor::default(),
                            TypeStyleToken(TypeScale::Label),
                            ForegroundRole(ColorRole::OnSurface),
                        ));
                        if !ids.is_empty() {
                            text.insert(ActionChip(ids));
                        }
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
                // A rebindable row is read through the registry: the source names its actions.
                for id in b.ids {
                    assert!(
                        text.contains(&format!("\"{id}\"")),
                        "{} lists the action {id} but {} never asks for it",
                        b.shown,
                        b.source
                    );
                }
                let defaults: Vec<KeyCode> = b
                    .ids
                    .iter()
                    .filter_map(|id| opts_bevy::action_keys::action(id))
                    .filter_map(|a| opts_bevy::KeyChord::parse(a.default).ok().flatten())
                    .map(|c| c.key)
                    .collect();
                for key in b.keys.iter().filter(|k| !defaults.contains(k)) {
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
        for (_, rows) in GROUPS {
            // A key may be listed once per group (W and S drive a bus and choose a tram's way).
            let mut seen = std::collections::HashSet::new();
            for b in *rows {
                assert!(!b.keys.is_empty() && !b.shown.is_empty(), "{b:?}");
                for key in b.keys {
                    assert!(seen.insert(*key), "{key:?} is listed twice");
                }
            }
        }
    }
}
