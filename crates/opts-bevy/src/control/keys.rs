//! Canonical key names of the control protocol and their Bevy equivalents.

use bevy::input::keyboard::{Key, KeyCode};
use smol_str::SmolStr;

const LETTERS: [KeyCode; 26] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
];

const DIGITS: [KeyCode; 10] = [
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

const FUNCTION: [(KeyCode, Key); 12] = [
    (KeyCode::F1, Key::F1),
    (KeyCode::F2, Key::F2),
    (KeyCode::F3, Key::F3),
    (KeyCode::F4, Key::F4),
    (KeyCode::F5, Key::F5),
    (KeyCode::F6, Key::F6),
    (KeyCode::F7, Key::F7),
    (KeyCode::F8, Key::F8),
    (KeyCode::F9, Key::F9),
    (KeyCode::F10, Key::F10),
    (KeyCode::F11, Key::F11),
    (KeyCode::F12, Key::F12),
];

/// Names that are not single characters or function keys.
const NAMED: [&str; 47] = [
    "Escape",
    "Enter",
    "Space",
    "Tab",
    "Backspace",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ShiftLeft",
    "ControlLeft",
    "AltLeft",
    "Period",
    "ShiftRight",
    "ControlRight",
    "AltRight",
    "Insert",
    "NumpadEnter",
    "Slash",
    "Comma",
    "Minus",
    "Equal",
    "BracketLeft",
    "BracketRight",
    "Semicolon",
    "Quote",
    "Backquote",
    "Backslash",
    "NumpadAdd",
    "NumpadSubtract",
    "NumpadMultiply",
    "NumpadDivide",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
];

fn named(name: &str) -> Option<(KeyCode, Key)> {
    Some(match name {
        "Escape" => (KeyCode::Escape, Key::Escape),
        "Enter" => (KeyCode::Enter, Key::Enter),
        "Space" => (KeyCode::Space, Key::Space),
        "Tab" => (KeyCode::Tab, Key::Tab),
        "Backspace" => (KeyCode::Backspace, Key::Backspace),
        "Delete" => (KeyCode::Delete, Key::Delete),
        "Home" => (KeyCode::Home, Key::Home),
        "End" => (KeyCode::End, Key::End),
        "PageUp" => (KeyCode::PageUp, Key::PageUp),
        "PageDown" => (KeyCode::PageDown, Key::PageDown),
        "ArrowUp" => (KeyCode::ArrowUp, Key::ArrowUp),
        "ArrowDown" => (KeyCode::ArrowDown, Key::ArrowDown),
        "ArrowLeft" => (KeyCode::ArrowLeft, Key::ArrowLeft),
        "ArrowRight" => (KeyCode::ArrowRight, Key::ArrowRight),
        "ShiftLeft" => (KeyCode::ShiftLeft, Key::Shift),
        "ControlLeft" => (KeyCode::ControlLeft, Key::Control),
        "AltLeft" => (KeyCode::AltLeft, Key::Alt),
        "Period" => (KeyCode::Period, Key::Character(".".into())),
        "ShiftRight" => (KeyCode::ShiftRight, Key::Shift),
        "ControlRight" => (KeyCode::ControlRight, Key::Control),
        "AltRight" => (KeyCode::AltRight, Key::Alt),
        "Insert" => (KeyCode::Insert, Key::Insert),
        "NumpadEnter" => (KeyCode::NumpadEnter, Key::Enter),
        "Slash" => (KeyCode::Slash, Key::Character("/".into())),
        "Comma" => (KeyCode::Comma, Key::Character(",".into())),
        "Minus" => (KeyCode::Minus, Key::Character("-".into())),
        "Equal" => (KeyCode::Equal, Key::Character("=".into())),
        "BracketLeft" => (KeyCode::BracketLeft, Key::Character("[".into())),
        "BracketRight" => (KeyCode::BracketRight, Key::Character("]".into())),
        "Semicolon" => (KeyCode::Semicolon, Key::Character(";".into())),
        "Quote" => (KeyCode::Quote, Key::Character("'".into())),
        "Backquote" => (KeyCode::Backquote, Key::Character("`".into())),
        "Backslash" => (KeyCode::Backslash, Key::Character("\\".into())),
        "NumpadAdd" => (KeyCode::NumpadAdd, Key::Character("+".into())),
        "NumpadSubtract" => (KeyCode::NumpadSubtract, Key::Character("-".into())),
        "NumpadMultiply" => (KeyCode::NumpadMultiply, Key::Character("*".into())),
        "NumpadDivide" => (KeyCode::NumpadDivide, Key::Character("/".into())),
        "Numpad0" => (KeyCode::Numpad0, Key::Character("0".into())),
        "Numpad1" => (KeyCode::Numpad1, Key::Character("1".into())),
        "Numpad2" => (KeyCode::Numpad2, Key::Character("2".into())),
        "Numpad3" => (KeyCode::Numpad3, Key::Character("3".into())),
        "Numpad4" => (KeyCode::Numpad4, Key::Character("4".into())),
        "Numpad5" => (KeyCode::Numpad5, Key::Character("5".into())),
        "Numpad6" => (KeyCode::Numpad6, Key::Character("6".into())),
        "Numpad7" => (KeyCode::Numpad7, Key::Character("7".into())),
        "Numpad8" => (KeyCode::Numpad8, Key::Character("8".into())),
        "Numpad9" => (KeyCode::Numpad9, Key::Character("9".into())),
        _ => return None,
    })
}

/// The physical and logical key for a protocol key name (case-insensitive for letters).
#[must_use]
pub fn parse_key(name: &str) -> Option<(KeyCode, Key)> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_lowercase() {
            let code = LETTERS[(lower as u8 - b'a') as usize];
            return Some((code, Key::Character(SmolStr::new(lower.to_string()))));
        }
        if let Some(d) = c.to_digit(10) {
            return Some((
                DIGITS[d as usize],
                Key::Character(SmolStr::new(c.to_string())),
            ));
        }
    }
    if let Some(n) = name
        .strip_prefix(['F', 'f'])
        .and_then(|n| n.parse::<usize>().ok())
    {
        if (1..=12).contains(&n) {
            return Some(FUNCTION[n - 1].clone());
        }
    }
    named(name).or_else(|| {
        NAMED
            .iter()
            .find(|k| k.eq_ignore_ascii_case(name))
            .and_then(|k| named(k))
    })
}

/// Every accepted name pattern, for error messages.
#[must_use]
pub fn key_names() -> Vec<String> {
    let mut v: Vec<String> = vec!["A-Z".into(), "0-9".into(), "F1-F12".into()];
    v.extend(NAMED.iter().map(|s| (*s).to_string()));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_function_keys_resolve() {
        assert_eq!(parse_key("W").map(|k| k.0), Some(KeyCode::KeyW));
        assert_eq!(parse_key("w").map(|k| k.0), Some(KeyCode::KeyW));
        assert_eq!(parse_key("5").map(|k| k.0), Some(KeyCode::Digit5));
        assert_eq!(parse_key("F1"), Some((KeyCode::F1, Key::F1)));
        assert_eq!(parse_key("f12").map(|k| k.0), Some(KeyCode::F12));
        assert_eq!(parse_key("Escape").map(|k| k.0), Some(KeyCode::Escape));
        assert_eq!(parse_key("escape").map(|k| k.0), Some(KeyCode::Escape));
        assert_eq!(parse_key("ArrowUp").map(|k| k.0), Some(KeyCode::ArrowUp));
    }

    #[test]
    fn unknown_names_do_not_resolve() {
        assert!(parse_key("F13").is_none());
        assert!(parse_key("Nope").is_none());
        assert!(parse_key("").is_none());
        assert!(!key_names().is_empty());
    }
}
