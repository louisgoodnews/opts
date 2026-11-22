//! Unit tests for `super` (`controls.rs`).

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

/// Keys a handler reads that the table does not list as keys of their own: the modifiers, which
/// the rows name in their chips ("Shift + Space", "Alt"), and Backspace, which a drive uses for the
/// vehicle and the UI uses for Back (open question FEAT-14 of the feature audit of 2026-11-18).
const NOT_LISTED: &[&str] = &[
    "ShiftLeft",
    "ShiftRight",
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "Backspace",
];

fn rows() -> impl Iterator<Item = &'static Binding> {
    GROUPS.iter().flat_map(|(_, rows)| rows.iter())
}

#[test]
fn every_action_of_the_registry_has_a_row() {
    for def in opts_bevy::action_keys::ACTIONS {
        assert!(
            rows().any(|b| b.ids.contains(&def.id)),
            "the action {} has no row in the controls table",
            def.id
        );
    }
}

/// Every handler file the table names. Fixed here, not taken from the rows: a file whose only row
/// is removed must still be checked, or its hard-wired keys go unlisted without a failure.
const HANDLERS: &[&str] = &[
    super::table::SIM,
    super::table::RAIL_TRACK,
    super::table::RAIL_CONTROLS,
    super::table::CAB_SWITCH,
    super::table::CAMERA,
    super::table::VIRTUAL_MIRRORS,
    super::table::EGO,
    super::table::PAUSE,
    super::table::SIM_PAUSE,
    super::table::HELP,
    super::table::THEME,
    super::table::DEV,
    super::table::MINIMAP,
    super::table::FULLMAP,
    super::table::DOOR_CAMERA,
    super::table::SESSION,
    super::table::QUICK_SAVE,
    super::table::ZONE_OVERLAY,
    super::table::LANE_OVERLAY,
];

#[test]
fn every_key_a_listed_handler_reads_has_a_row() {
    for source in HANDLERS {
        let source: &str = source;
        let text = std::fs::read_to_string(root().join(source))
            .unwrap_or_else(|e| panic!("{source}: {e}"));
        let listed: Vec<String> = rows()
            .filter(|b| b.source == source)
            .flat_map(|b| b.keys.iter())
            .map(|k| format!("{k:?}"))
            .collect();
        for key in key_names_read(&text) {
            assert!(
                NOT_LISTED.contains(&key.as_str()) || listed.contains(&key),
                "{source} reads KeyCode::{key} but no row of the controls table lists it"
            );
        }
    }
}

/// The names that follow each `KeyCode::` in `text`.
fn key_names_read(text: &str) -> Vec<String> {
    text.split("KeyCode::")
        .skip(1)
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect()
        })
        .collect()
}
