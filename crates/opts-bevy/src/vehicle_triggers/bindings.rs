//! DirectInput scan code mapping and key chord parsing for vehicle script bindings.

use super::*;
mod rows;
mod scan;

pub use scan::dik_to_keycode;

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
