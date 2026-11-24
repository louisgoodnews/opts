//! The chips of the Help controls table against the registry (M-0164, ISS-015): a rebindable row
//! prints the default chord of each of its actions and nothing else, and lists the chord's key.

use std::collections::HashMap;

use bevy::prelude::KeyCode;
use opts_bevy::action_keys::{action, display_on};
use opts_bevy::KeyChord;
use opts_game::help::controls::GROUPS;

/// The letters of the keyboard the rows are printed for: a German layout, where the key at the Z
/// position says Y (the rail lever row prints "Q / Y").
fn german_letters() -> HashMap<KeyCode, char> {
    [(KeyCode::KeyZ, 'Y'), (KeyCode::KeyY, 'Z')]
        .into_iter()
        .collect()
}

/// The texts a chip may print for `chord`: the registry's print ([`display_on`]) and, for the key
/// at the Equal position, the "+" of the zoom row (Shift+= on a US keyboard).
fn accepted_prints(chord: KeyChord) -> Vec<String> {
    let mut out: Vec<String> = display_on(chord, &german_letters()).into_iter().collect();
    if chord.key == KeyCode::Equal && !chord.shift && !chord.ctrl {
        out.push("+".to_owned());
    }
    out
}

#[test]
fn every_chip_of_a_rebindable_row_names_the_chord_of_its_action() {
    for (_, rows) in GROUPS {
        for b in rows.iter().filter(|b| !b.ids.is_empty()) {
            let tokens: Vec<&str> = b
                .shown
                .split(|c: char| c == '/' || c.is_whitespace())
                .filter(|t| !t.is_empty())
                .collect();
            let mut printed_by_action = Vec::new();
            for id in b.ids {
                let def = action(id)
                    .unwrap_or_else(|| panic!("{}: no action {id} in the registry", b.shown));
                let chord = KeyChord::parse(def.default)
                    .unwrap_or_else(|e| panic!("{id}: default {} is not a chord: {e}", def.default))
                    .unwrap_or_else(|| panic!("{id}: default is none"));
                let prints = accepted_prints(chord);
                assert!(
                    tokens.iter().any(|t| prints.iter().any(|p| p == t)),
                    "the chip \"{}\" does not print {id} (default {}, printed {prints:?})",
                    b.shown,
                    def.default
                );
                assert!(
                    b.keys.contains(&chord.key),
                    "the row \"{}\" does not list {:?}, the key of {id}",
                    b.shown,
                    chord.key
                );
                printed_by_action.push(prints);
            }
            for t in &tokens {
                assert!(
                    printed_by_action
                        .iter()
                        .any(|prints| prints.iter().any(|p| p == t)),
                    "the chip \"{}\" prints {t}, which none of its actions {:?} is bound to",
                    b.shown,
                    b.ids
                );
            }
        }
    }
}
