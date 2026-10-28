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
    /// Which vehicles the action is for.
    pub scope: ActionScope,
}

/// The vehicles an action is for: a key of the rail controls may share its default with a road
/// vehicle's key (Q is the bus microphone and the tram's lever).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionScope {
    /// Every vehicle.
    All,
    /// Road vehicles (buses, trolleybuses).
    Road,
    /// Rail vehicles (trams, trains).
    Rail,
}

const fn def(group: &'static str, id: &'static str, default: &'static str) -> ActionDef {
    ActionDef {
        id,
        group,
        default,
        scope: ActionScope::All,
    }
}

const fn road(group: &'static str, id: &'static str, default: &'static str) -> ActionDef {
    ActionDef {
        id,
        group,
        default,
        scope: ActionScope::Road,
    }
}

const fn rail(group: &'static str, id: &'static str, default: &'static str) -> ActionDef {
    ActionDef {
        id,
        group,
        default,
        scope: ActionScope::Rail,
    }
}

/// The actions, in the order of the settings page.
pub const ACTIONS: &[ActionDef] = &[
    road("drive", "parking_brake", "Period"),
    // The controls of rail vehicles: no pedals and no parking brake. W and S choose the
    // direction of travel, Q and Y (the key at the Z position of a QWERTY keyboard) move the
    // power lever up (towards traction) and down (towards braking), the arrows choose the way at
    // the next point.
    rail("drive", "rail_forward", "W"),
    rail("drive", "rail_reverse", "S"),
    rail("drive", "rail_lever_up", "Q"),
    rail("drive", "rail_lever_down", "Z"),
    rail("drive", "rail_lever_zero", "X"),
    rail("drive", "rail_switch_toggle", "G"),
    // The other cab of a tram (A and B) or of a traction of trains.
    rail("drive", "rail_switch_cab", "F3"),
    rail("drive", "rail_branch_left", "ArrowLeft"),
    rail("drive", "rail_branch_straight", "ArrowUp"),
    rail("drive", "rail_branch_right", "ArrowRight"),
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
    def("ui", "developer_panel_detail", "Shift+F10"),
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

/// The chord as a player reads it: `Ctrl+Shift+` and the key as printed on it (`.` for Period).
#[must_use]
pub fn display(chord: KeyChord) -> Option<String> {
    display_on(chord, &HashMap::new())
}

/// [`display`] with the letters the keys of this keyboard really show.
#[must_use]
pub fn display_on(chord: KeyChord, letters: &HashMap<KeyCode, char>) -> Option<String> {
    let text = chord.to_text()?;
    if let Some(letter) = letters.get(&chord.key) {
        let prefix_len = text.len() - crate::control::keys::key_name(chord.key)?.len();
        return Some(format!("{}{letter}", &text[..prefix_len]));
    }
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

/// Learns what the keys of the player's keyboard say: the key events carry the logical key next
/// to the physical one (a German keyboard's Y arrives as the physical `KeyZ`).
pub fn learn_key_layout(
    events: Option<MessageReader<bevy::input::keyboard::KeyboardInput>>,
    mut keys: ResMut<ActionKeys>,
) {
    let Some(mut events) = events else { return };
    use bevy::input::keyboard::Key;
    for e in events.read() {
        if !e.state.is_pressed() {
            continue;
        }
        let Key::Character(text) = &e.logical_key else {
            continue;
        };
        let mut chars = text.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            continue;
        };
        if !c.is_ascii_alphabetic() {
            continue;
        }
        // Looking must not wake every system that watches the registry: only a change does.
        if keys.bypass_change_detection().learn_letter(e.key_code, c) {
            keys.set_changed();
            tracing::info!(physical = ?e.key_code, printed = %c.to_ascii_uppercase(), "keyboard layout learned: the key says another letter than its US name");
        }
    }
}

#[cfg(test)]
mod tests;
