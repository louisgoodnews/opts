//! The built-in keyboard actions (M-0142): one registry of what a key does outside the vehicle's
//! own triggers, with the default chord and the chord the player chose.
//!
//! The input systems ask [`ActionKeys`] (`just_pressed("help")`) instead of naming a `KeyCode`,
//! so a rebinding takes effect at once and the hints can print the key the player really has.
//! The game fills the chords from the settings `controls.<group>.<id>`; without it the defaults
//! stand. Matching is exact on Ctrl and Shift: Shift+F7 is not F7.

use std::collections::HashMap;

use bevy::input::ButtonInput;
use bevy::prelude::*;

use crate::vehicle_triggers::KeyChord;
mod catalog;
mod layout;

pub use catalog::*;
pub use layout::*;

/// The current chord of every action.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActionKeys {
    chords: HashMap<&'static str, Option<KeyChord>>,
    /// The letter printed on a key where it differs from the US name of its position (the key at
    /// the Z position of a German keyboard says Y): learned from the key events, or guessed
    /// from the system locale until the key was pressed.
    letters: HashMap<KeyCode, char>,
}

impl Default for ActionKeys {
    fn default() -> Self {
        Self {
            chords: ACTIONS
                .iter()
                .map(|a| (a.id, KeyChord::parse(a.default).ok().flatten()))
                .collect(),
            letters: HashMap::new(),
        }
    }
}

impl ActionKeys {
    /// The defaults with the keyboard layout guessed from the system locale: Y and Z swap their
    /// printed letters for a German locale until a key event tells what a key really says.
    #[must_use]
    pub fn with_system_layout() -> Self {
        let mut keys = Self::default();
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
            .unwrap_or_default();
        if locale.to_ascii_lowercase().starts_with("de") {
            keys.letters.insert(KeyCode::KeyZ, 'Y');
            keys.letters.insert(KeyCode::KeyY, 'Z');
        }
        keys
    }

    /// Learns the letter printed on `key` (as the key events report it); returns whether the
    /// known letters changed.
    pub fn learn_letter(&mut self, key: KeyCode, printed: char) -> bool {
        let printed = printed.to_ascii_uppercase();
        let us = crate::control::keys::key_name(key)
            .and_then(|n| n.chars().next().filter(|_| n.len() == 1))
            .unwrap_or(printed);
        if printed == us {
            self.letters.remove(&key).is_some()
        } else {
            self.letters.insert(key, printed) != Some(printed)
        }
    }

    /// The chord of `id`, `None` when unbound (or unknown).
    #[must_use]
    pub fn chord(&self, id: &str) -> Option<KeyChord> {
        self.chords.get(id).copied().flatten()
    }

    /// Sets the chord of `id` (`None` unbinds it).
    pub fn set(&mut self, id: &str, chord: Option<KeyChord>) {
        if let Some(a) = action(id) {
            self.chords.insert(a.id, chord);
        }
    }

    /// The chord as printed in hints and on the page (`F7`, `Shift+F7`), `none` when unbound.
    #[must_use]
    pub fn label(&self, id: &str) -> String {
        self.chord(id)
            .and_then(|c| display_on(c, &self.letters))
            .unwrap_or_else(|| "none".to_owned())
    }

    /// Whether the key of `id` went down this frame with exactly its modifiers.
    #[must_use]
    pub fn just_pressed(&self, keys: &ButtonInput<KeyCode>, id: &str) -> bool {
        self.chord(id)
            .is_some_and(|c| keys.just_pressed(c.key) && modifiers_match(keys, c))
    }

    /// Whether the key of `id` is held with exactly its modifiers.
    #[must_use]
    pub fn pressed(&self, keys: &ButtonInput<KeyCode>, id: &str) -> bool {
        self.chord(id)
            .is_some_and(|c| keys.pressed(c.key) && modifiers_match(keys, c))
    }

    /// The id of the action that has `chord`, other than `except`.
    #[must_use]
    pub fn holder_of(&self, chord: KeyChord, except: &str) -> Option<&'static str> {
        ACTIONS
            .iter()
            .find(|a| a.id != except && self.chord(a.id) == Some(chord))
            .map(|a| a.id)
    }
}

/// The registry of the app, or the defaults where a test or a headless app has none.
#[must_use]
pub fn resolve<'a>(keys: &'a Option<Res<ActionKeys>>) -> &'a ActionKeys {
    static DEFAULTS: std::sync::OnceLock<ActionKeys> = std::sync::OnceLock::new();
    match keys {
        Some(k) => k,
        None => DEFAULTS.get_or_init(ActionKeys::default),
    }
}

#[cfg(test)]
mod tests;
