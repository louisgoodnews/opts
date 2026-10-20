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

/// One built-in action.
#[derive(Debug, Clone, Copy)]
pub struct ActionDef {
    /// Short name (`help`), the argument of [`ActionKeys`] and the last part of the setting.
    pub id: &'static str,
    /// Group (`ui`, `map`, `drive`), the middle part of the setting `controls.<group>.<id>`.
    pub group: &'static str,
    /// The default chord as setting text.
    pub default: &'static str,
}

const fn def(group: &'static str, id: &'static str, default: &'static str) -> ActionDef {
    ActionDef { id, group, default }
}

/// The actions, in the order of the settings page.
pub const ACTIONS: &[ActionDef] = &[
    def("drive", "parking_brake", "Period"),
    def("map", "minimap_toggle", "F7"),
    def("map", "minimap_rotate", "Shift+F7"),
    def("map", "minimap_speed_zoom", "Ctrl+F7"),
    def("map", "minimap_zoom_in", "Equal"),
    def("map", "minimap_zoom_out", "Minus"),
    def("map", "full_map", "F6"),
    def("camera", "door_camera", "F4"),
    def("ui", "help", "F1"),
    def("ui", "theme", "F2"),
    def("ui", "quick_save", "F5"),
    def("ui", "quick_load", "F9"),
    def("ui", "switch_vehicle", "F8"),
    def("ui", "switch_vehicle_prev", "Shift+F8"),
    def("ui", "developer_panel", "F10"),
];

/// The setting that holds the chord of an action.
#[must_use]
pub fn setting_of(def: &ActionDef) -> String {
    format!("controls.{}.{}", def.group, def.id)
}

/// The definition of `id`.
#[must_use]
pub fn action(id: &str) -> Option<&'static ActionDef> {
    ACTIONS.iter().find(|a| a.id == id)
}

/// The current chord of every action.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActionKeys {
    chords: HashMap<&'static str, Option<KeyChord>>,
}

impl Default for ActionKeys {
    fn default() -> Self {
        Self {
            chords: ACTIONS
                .iter()
                .map(|a| (a.id, KeyChord::parse(a.default).ok().flatten()))
                .collect(),
        }
    }
}

impl ActionKeys {
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
            .and_then(display)
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

/// The chord as a player reads it: `Ctrl+Shift+` and the key as printed on it (`.` for Period).
#[must_use]
pub fn display(chord: KeyChord) -> Option<String> {
    let text = chord.to_text()?;
    let prefix_len = text.len() - crate::control::keys::key_name(chord.key)?.len();
    let key = match chord.key {
        KeyCode::Period => ".",
        KeyCode::Comma => ",",
        KeyCode::Minus => "-",
        KeyCode::Equal => "=",
        KeyCode::Slash => "/",
        KeyCode::Backslash => "\\",
        KeyCode::Semicolon => ";",
        KeyCode::Quote => "'",
        KeyCode::BracketLeft => "[",
        KeyCode::BracketRight => "]",
        KeyCode::Backquote => "`",
        KeyCode::ArrowUp => "↑",
        KeyCode::ArrowDown => "↓",
        KeyCode::ArrowLeft => "←",
        KeyCode::ArrowRight => "→",
        _ => return Some(text),
    };
    Some(format!("{}{key}", &text[..prefix_len]))
}

fn modifiers_match(keys: &ButtonInput<KeyCode>, c: KeyChord) -> bool {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    shift == c.shift && ctrl == c.ctrl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_and_are_unique() {
        let keys = ActionKeys::default();
        let mut seen = std::collections::HashSet::new();
        for a in ACTIONS {
            let chord = keys
                .chord(a.id)
                .unwrap_or_else(|| panic!("{} parses", a.id));
            assert_eq!(chord.to_text().as_deref(), Some(a.default), "{}", a.id);
            assert!(seen.insert(a.default), "{} duplicates {}", a.id, a.default);
            assert!(!a.id.is_empty() && !a.group.is_empty());
        }
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let keys = ActionKeys::default();
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::F7);
        assert!(keys.just_pressed(&input, "minimap_toggle"));
        assert!(!keys.just_pressed(&input, "minimap_rotate"));
        input.clear();
        input.release(KeyCode::F7);
        input.clear();
        input.press(KeyCode::ShiftLeft);
        input.press(KeyCode::F7);
        assert!(keys.just_pressed(&input, "minimap_rotate"));
        assert!(!keys.just_pressed(&input, "minimap_toggle"));
    }

    #[test]
    fn a_rebinding_moves_the_key_and_unbinding_silences_it() {
        let mut keys = ActionKeys::default();
        keys.set("help", KeyChord::parse("Ctrl+H").unwrap());
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::F1);
        assert!(!keys.just_pressed(&input, "help"));
        input.clear();
        input.release(KeyCode::F1);
        input.clear();
        input.press(KeyCode::ControlLeft);
        input.press(KeyCode::KeyH);
        assert!(keys.just_pressed(&input, "help"));
        assert_eq!(keys.label("help"), "Ctrl+H");
        keys.set("help", None);
        assert!(!keys.just_pressed(&input, "help"));
        assert_eq!(keys.label("help"), "none");
    }

    #[test]
    fn labels_print_the_key_as_it_is_on_the_keyboard() {
        let mut keys = ActionKeys::default();
        assert_eq!(keys.label("parking_brake"), ".");
        assert_eq!(keys.label("minimap_zoom_in"), "=");
        keys.set("help", KeyChord::parse("Ctrl+Shift+Slash").unwrap());
        assert_eq!(keys.label("help"), "Ctrl+Shift+/");
        assert_eq!(keys.label("minimap_rotate"), "Shift+F7");
    }

    #[test]
    fn the_holder_of_a_chord_is_named() {
        let keys = ActionKeys::default();
        let f6 = KeyChord::parse("F6").unwrap().unwrap();
        assert_eq!(keys.holder_of(f6, "help"), Some("full_map"));
        assert_eq!(keys.holder_of(f6, "full_map"), None);
    }
}
