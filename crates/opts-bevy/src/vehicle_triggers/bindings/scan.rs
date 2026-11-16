//! DirectInput scan code to key: the base keys and the extended ones.

use bevy::prelude::KeyCode;

use super::rows::{DIGITS, FKEYS, LETTERS_A, LETTERS_Q, LETTERS_Z};

fn dik_to_keycode_base(code: u16) -> Option<KeyCode> {
    use KeyCode as K;
    let c = usize::from(code);
    Some(match code {
        1 => K::Escape,
        2..=11 => DIGITS[c - 2],
        12 => K::Minus,
        13 => K::Equal,
        14 => K::Backspace,
        15 => K::Tab,
        16..=25 => LETTERS_Q[c - 16],
        26 => K::BracketLeft,
        27 => K::BracketRight,
        28 => K::Enter,
        29 => K::ControlLeft,
        30..=38 => LETTERS_A[c - 30],
        39 => K::Semicolon,
        40 => K::Quote,
        41 => K::Backquote,
        42 => K::ShiftLeft,
        43 => K::Backslash,
        44..=50 => LETTERS_Z[c - 44],
        51 => K::Comma,
        52 => K::Period,
        53 => K::Slash,
        54 => K::ShiftRight,
        55 => K::NumpadMultiply,
        56 => K::AltLeft,
        57 => K::Space,
        58 => K::CapsLock,
        59..=68 => FKEYS[c - 59],
        69 => K::NumLock,
        70 => K::ScrollLock,
        71 => K::Numpad7,
        72 => K::Numpad8,
        73 => K::Numpad9,
        74 => K::NumpadSubtract,
        75 => K::Numpad4,
        76 => K::Numpad5,
        77 => K::Numpad6,
        78 => K::NumpadAdd,
        79 => K::Numpad1,
        80 => K::Numpad2,
        81 => K::Numpad3,
        82 => K::Numpad0,
        83 => K::NumpadDecimal,
        86 => K::IntlBackslash,
        87 => K::F11,
        88 => K::F12,
        _ => return None,
    })
}

fn dik_to_keycode_ext(code: u16) -> Option<KeyCode> {
    use KeyCode as K;
    Some(match code {
        156 => K::NumpadEnter,
        157 => K::ControlRight,
        181 => K::NumpadDivide,
        183 => K::PrintScreen,
        184 => K::AltRight,
        197 => K::Pause,
        199 => K::Home,
        200 => K::ArrowUp,
        201 => K::PageUp,
        203 => K::ArrowLeft,
        205 => K::ArrowRight,
        207 => K::End,
        208 => K::ArrowDown,
        209 => K::PageDown,
        210 => K::Insert,
        211 => K::Delete,
        219 => K::SuperLeft,
        220 => K::SuperRight,
        221 => K::ContextMenu,
        _ => return None,
    })
}

#[must_use]
pub fn dik_to_keycode(code: u16) -> Option<KeyCode> {
    dik_to_keycode_base(code).or_else(|| dik_to_keycode_ext(code))
}
