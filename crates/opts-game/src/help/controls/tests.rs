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
