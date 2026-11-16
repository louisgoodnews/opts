//! Showing chords as players read them, and learning the keyboard layout from the letters pressed.

use super::*;

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

pub(super) fn modifiers_match(keys: &ButtonInput<KeyCode>, c: KeyChord) -> bool {
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
