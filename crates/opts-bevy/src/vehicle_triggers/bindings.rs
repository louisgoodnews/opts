//! DirectInput scan code mapping and key chord parsing for vehicle script bindings.

use super::*;

/// Keys OPTS binds in-game (driving, gears, cameras, overlay, reporting). OMSI bindings on these
/// keys are dropped unless they require Ctrl; a rebinding to one of them is refused (A, E, Q
/// and 6-9 are free: the steering is on the arrows, the free camera is gated off the vehicle).
pub const OPTS_RESERVED_KEYS: &[KeyCode] = &[
    KeyCode::KeyW,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyN,
    KeyCode::KeyR,
    KeyCode::Space,
    // The viewpoint editor (Shift + Tab; Home resets inside it) and the virtual mirrors (Tab).
    KeyCode::Tab,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F8,
    // The mini map: F7 shows it, Shift+F7 turns it, + and - zoom. F9 is the quick load.
    KeyCode::F7,
    KeyCode::F9,
    KeyCode::Minus,
    KeyCode::Equal,
    // The door camera cycles its door with F4 (M-0109); the full-size map is F6 (M-0108; M stays
    // the engine start of OMSI).
    KeyCode::F4,
    KeyCode::F6,
    KeyCode::F10,
    KeyCode::F12,
    KeyCode::Escape,
];

// The key rows of the DirectInput scan codes (`DIK_*`), by code from the first key of the row.
mod rows {
    use bevy::prelude::KeyCode as K;
    use bevy::prelude::KeyCode;

    pub(super) const LETTERS_Q: [KeyCode; 10] = [
        K::KeyQ,
        K::KeyW,
        K::KeyE,
        K::KeyR,
        K::KeyT,
        K::KeyY,
        K::KeyU,
        K::KeyI,
        K::KeyO,
        K::KeyP,
    ];
    pub(super) const LETTERS_A: [KeyCode; 9] = [
        K::KeyA,
        K::KeyS,
        K::KeyD,
        K::KeyF,
        K::KeyG,
        K::KeyH,
        K::KeyJ,
        K::KeyK,
        K::KeyL,
    ];
    pub(super) const LETTERS_Z: [KeyCode; 7] = [
        K::KeyZ,
        K::KeyX,
        K::KeyC,
        K::KeyV,
        K::KeyB,
        K::KeyN,
        K::KeyM,
    ];
    pub(super) const DIGITS: [KeyCode; 10] = [
        K::Digit1,
        K::Digit2,
        K::Digit3,
        K::Digit4,
        K::Digit5,
        K::Digit6,
        K::Digit7,
        K::Digit8,
        K::Digit9,
        K::Digit0,
    ];
    pub(super) const FKEYS: [KeyCode; 10] = [
        K::F1,
        K::F2,
        K::F3,
        K::F4,
        K::F5,
        K::F6,
        K::F7,
        K::F8,
        K::F9,
        K::F10,
    ];
}
use rows::{DIGITS, FKEYS, LETTERS_A, LETTERS_Q, LETTERS_Z};


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

/// The physical key of a DirectInput scan code (the numbering of `keyboard.cfg` and `.kyb`).
#[must_use]
pub fn dik_to_keycode(code: u16) -> Option<KeyCode> {
    dik_to_keycode_base(code).or_else(|| dik_to_keycode_ext(code))
}

/// A key that raises a script trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptKeyBinding {
    /// Trigger name.
    pub trigger: String,
    /// Physical key.
    pub key: KeyCode,
    /// Shift must be held.
    pub shift: bool,
    /// Ctrl must be held.
    pub ctrl: bool,
    /// Releasing the key raises `<trigger>_off`.
    pub held: bool,
}

/// A key with the modifiers that must be held with it (the value of a key binding setting).
///
/// Stored as text: `none`, or an optional `Ctrl+` and `Shift+` followed by a protocol key name
/// (`L`, `7`, `F5`, `NumpadDivide`; see [`crate::control::keys::parse_key`]). Keys are physical,
/// so a binding keeps its position on other keyboard layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChord {
    /// The physical key.
    pub key: KeyCode,
    /// Shift must be held.
    pub shift: bool,
    /// Ctrl must be held.
    pub ctrl: bool,
}

impl KeyChord {
    /// Parses `text`; `Ok(None)` is `none` (or empty: unbound).
    ///
    /// # Errors
    /// The message says what is wrong with `text`.
    pub fn parse(text: &str) -> Result<Option<Self>, String> {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("none") {
            return Ok(None);
        }
        let (mut shift, mut ctrl) = (false, false);
        let mut rest = text;
        while let Some((head, tail)) = rest.split_once('+') {
            if tail.is_empty() {
                break;
            }
            match head.trim().to_ascii_lowercase().as_str() {
                "shift" => shift = true,
                "ctrl" | "control" | "strg" => ctrl = true,
                other => return Err(format!("unknown modifier \"{other}\" (Ctrl, Shift)")),
            }
            rest = tail;
        }
        let rest = rest.trim();
        let Some((key, _)) = crate::control::keys::parse_key(rest) else {
            return Err(format!(
                "unknown key \"{rest}\" (A-Z, 0-9, F1-F12, Numpad0-9, NumpadDivide, ... or none)"
            ));
        };
        Ok(Some(Self { key, shift, ctrl }))
    }

    /// The canonical text (`Ctrl+Shift+L`), `None` if the key has no protocol name.
    #[must_use]
    pub fn to_text(self) -> Option<String> {
        let name = crate::control::keys::key_name(self.key)?;
        let mut out = String::new();
        if self.ctrl {
            out.push_str("Ctrl+");
        }
        if self.shift {
            out.push_str("Shift+");
        }
        out.push_str(&name);
        Some(out)
    }

    /// Whether OPTS itself uses this key combination (see [`OPTS_RESERVED_KEYS`]).
    #[must_use]
    pub fn is_reserved(self) -> bool {
        !self.ctrl && OPTS_RESERVED_KEYS.contains(&self.key)
    }
}
