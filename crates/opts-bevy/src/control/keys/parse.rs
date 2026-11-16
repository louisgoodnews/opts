//! Parsing key names into key codes: the named keys and the parser behind the control protocol.

use super::*;

use super::tables::*;


pub(super) fn named(name: &str) -> Option<(KeyCode, Key)> {
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
        "NumpadDecimal" => (KeyCode::NumpadDecimal, Key::Character(".".into())),
        "ScrollLock" => (KeyCode::ScrollLock, Key::ScrollLock),
        "CapsLock" => (KeyCode::CapsLock, Key::CapsLock),
        "Pause" => (KeyCode::Pause, Key::Pause),
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
