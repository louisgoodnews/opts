//! System keyboard layout detection and key mapping.
//!
//! Detects system keyboard layout (e.g. German QWERTZ vs US QWERTY) from Linux
//! system configuration (`/etc/default/keyboard`, `setxkbmap`, environment variables)
//! and provides character translation for search inputs and text spinboxes.

use std::fs;
use std::process::Command;

use bevy::prelude::*;

/// Supported high-level keyboard layout families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardLayoutFamily {
    #[default]
    Qwerty,
    Qwertz,
    Azerty,
}

/// Resource caching the detected system keyboard configuration.
#[derive(Resource, Debug, Clone)]
pub struct SystemKeyboardLayout {
    pub family: KeyboardLayoutFamily,
    pub layout_code: String,
    pub model: String,
}

impl Default for SystemKeyboardLayout {
    fn default() -> Self {
        Self::detect()
    }
}

impl SystemKeyboardLayout {
    /// Detects system keyboard layout by querying system config files and tools.
    pub fn detect() -> Self {
        let mut layout_code = String::new();
        let mut model = String::new();

        // 1. Try reading /etc/default/keyboard
        if let Ok(content) = fs::read_to_string("/etc/default/keyboard") {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(val) = trimmed.strip_prefix("XKBLAYOUT=") {
                    layout_code = val.trim_matches('"').trim_matches('\'').to_string();
                } else if let Some(val) = trimmed.strip_prefix("XKBMODEL=") {
                    model = val.trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }

        // 2. Try setxkbmap -query if layout_code is still empty or to verify runtime layout
        if layout_code.is_empty() {
            if let Ok(output) = Command::new("setxkbmap").arg("-query").output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if let Some(rest) = trimmed.strip_prefix("layout:") {
                            layout_code = rest.trim().to_string();
                        } else if let Some(rest) = trimmed.strip_prefix("model:") {
                            model = rest.trim().to_string();
                        }
                    }
                }
            }
        }

        // 3. Fallback to locale / environment variables
        if layout_code.is_empty() {
            if let Ok(xkb) = std::env::var("XKB_DEFAULT_LAYOUT") {
                layout_code = xkb;
            } else if let Ok(lang) = std::env::var("LANG").or_else(|_| std::env::var("LC_ALL")) {
                if lang.starts_with("de") || lang.starts_with("at") || lang.starts_with("ch") {
                    layout_code = "de".to_string();
                } else if lang.starts_with("fr") {
                    layout_code = "fr".to_string();
                } else {
                    layout_code = "us".to_string();
                }
            } else {
                layout_code = "us".to_string();
            }
        }

        let first_layout = layout_code
            .split(',')
            .next()
            .unwrap_or("us")
            .trim()
            .to_lowercase();

        let family = match first_layout.as_str() {
            "de" | "at" | "ch" | "cz" | "hu" | "pl" | "ro" | "sk" => KeyboardLayoutFamily::Qwertz,
            "fr" | "be" => KeyboardLayoutFamily::Azerty,
            _ => KeyboardLayoutFamily::Qwerty,
        };

        tracing::info!(
            "Detected system keyboard layout: code='{}', family={:?}, model='{}'",
            layout_code,
            family,
            model
        );

        Self {
            family,
            layout_code,
            model,
        }
    }

    /// Maps a Bevy [`KeyCode`] and shift state to the appropriate character according to the layout.
    pub fn map_key_to_char(&self, key: KeyCode, shift: bool) -> Option<char> {
        match self.family {
            KeyboardLayoutFamily::Qwertz => map_qwertz(key, shift),
            KeyboardLayoutFamily::Azerty => map_azerty(key, shift),
            KeyboardLayoutFamily::Qwerty => map_qwerty(key, shift),
        }
    }
}

fn map_qwertz(key: KeyCode, shift: bool) -> Option<char> {
    match key {
        // Z and Y are swapped on QWERTZ
        KeyCode::KeyY => Some(if shift { 'Z' } else { 'z' }),
        KeyCode::KeyZ => Some(if shift { 'Y' } else { 'y' }),

        // German Umlauts and symbols on physical keys
        KeyCode::BracketLeft => Some(if shift { 'Ü' } else { 'ü' }),
        KeyCode::Semicolon => Some(if shift { 'Ö' } else { 'ö' }),
        KeyCode::Quote => Some(if shift { 'Ä' } else { 'ä' }),
        KeyCode::Minus => Some(if shift { '?' } else { 'ß' }),
        KeyCode::Slash => Some(if shift { '_' } else { '-' }),

        // Standard letters
        KeyCode::KeyA => Some(if shift { 'A' } else { 'a' }),
        KeyCode::KeyB => Some(if shift { 'B' } else { 'b' }),
        KeyCode::KeyC => Some(if shift { 'C' } else { 'c' }),
        KeyCode::KeyD => Some(if shift { 'D' } else { 'd' }),
        KeyCode::KeyE => Some(if shift { 'E' } else { 'e' }),
        KeyCode::KeyF => Some(if shift { 'F' } else { 'f' }),
        KeyCode::KeyG => Some(if shift { 'G' } else { 'g' }),
        KeyCode::KeyH => Some(if shift { 'H' } else { 'h' }),
        KeyCode::KeyI => Some(if shift { 'I' } else { 'i' }),
        KeyCode::KeyJ => Some(if shift { 'J' } else { 'j' }),
        KeyCode::KeyK => Some(if shift { 'K' } else { 'k' }),
        KeyCode::KeyL => Some(if shift { 'L' } else { 'l' }),
        KeyCode::KeyM => Some(if shift { 'M' } else { 'm' }),
        KeyCode::KeyN => Some(if shift { 'N' } else { 'n' }),
        KeyCode::KeyO => Some(if shift { 'O' } else { 'o' }),
        KeyCode::KeyP => Some(if shift { 'P' } else { 'p' }),
        KeyCode::KeyQ => Some(if shift { 'Q' } else { 'q' }),
        KeyCode::KeyR => Some(if shift { 'R' } else { 'r' }),
        KeyCode::KeyS => Some(if shift { 'S' } else { 's' }),
        KeyCode::KeyT => Some(if shift { 'T' } else { 't' }),
        KeyCode::KeyU => Some(if shift { 'U' } else { 'u' }),
        KeyCode::KeyV => Some(if shift { 'V' } else { 'v' }),
        KeyCode::KeyW => Some(if shift { 'W' } else { 'w' }),
        KeyCode::KeyX => Some(if shift { 'X' } else { 'x' }),

        // Digits
        KeyCode::Digit0 => Some('0'),
        KeyCode::Digit1 => Some('1'),
        KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'),
        KeyCode::Digit4 => Some('4'),
        KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'),
        KeyCode::Digit7 => Some('7'),
        KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        KeyCode::Space => Some(' '),
        _ => None,
    }
}

fn map_azerty(key: KeyCode, shift: bool) -> Option<char> {
    match key {
        KeyCode::KeyA => Some(if shift { 'Q' } else { 'q' }),
        KeyCode::KeyQ => Some(if shift { 'A' } else { 'a' }),
        KeyCode::KeyZ => Some(if shift { 'W' } else { 'w' }),
        KeyCode::KeyW => Some(if shift { 'Z' } else { 'z' }),
        KeyCode::KeyM => Some(if shift { '?' } else { ',' }),
        KeyCode::Digit0 => Some('0'),
        KeyCode::Digit1 => Some('1'),
        KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'),
        KeyCode::Digit4 => Some('4'),
        KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'),
        KeyCode::Digit7 => Some('7'),
        KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        KeyCode::Space => Some(' '),
        _ => map_qwerty(key, shift),
    }
}

fn map_qwerty(key: KeyCode, shift: bool) -> Option<char> {
    match key {
        KeyCode::KeyA => Some(if shift { 'A' } else { 'a' }),
        KeyCode::KeyB => Some(if shift { 'B' } else { 'b' }),
        KeyCode::KeyC => Some(if shift { 'C' } else { 'c' }),
        KeyCode::KeyD => Some(if shift { 'D' } else { 'd' }),
        KeyCode::KeyE => Some(if shift { 'E' } else { 'e' }),
        KeyCode::KeyF => Some(if shift { 'F' } else { 'f' }),
        KeyCode::KeyG => Some(if shift { 'G' } else { 'g' }),
        KeyCode::KeyH => Some(if shift { 'H' } else { 'h' }),
        KeyCode::KeyI => Some(if shift { 'I' } else { 'i' }),
        KeyCode::KeyJ => Some(if shift { 'J' } else { 'j' }),
        KeyCode::KeyK => Some(if shift { 'K' } else { 'k' }),
        KeyCode::KeyL => Some(if shift { 'L' } else { 'l' }),
        KeyCode::KeyM => Some(if shift { 'M' } else { 'm' }),
        KeyCode::KeyN => Some(if shift { 'N' } else { 'n' }),
        KeyCode::KeyO => Some(if shift { 'O' } else { 'o' }),
        KeyCode::KeyP => Some(if shift { 'P' } else { 'p' }),
        KeyCode::KeyQ => Some(if shift { 'Q' } else { 'q' }),
        KeyCode::KeyR => Some(if shift { 'R' } else { 'r' }),
        KeyCode::KeyS => Some(if shift { 'S' } else { 's' }),
        KeyCode::KeyT => Some(if shift { 'T' } else { 't' }),
        KeyCode::KeyU => Some(if shift { 'U' } else { 'u' }),
        KeyCode::KeyV => Some(if shift { 'V' } else { 'v' }),
        KeyCode::KeyW => Some(if shift { 'W' } else { 'w' }),
        KeyCode::KeyX => Some(if shift { 'X' } else { 'x' }),
        KeyCode::KeyY => Some(if shift { 'Y' } else { 'y' }),
        KeyCode::KeyZ => Some(if shift { 'Z' } else { 'z' }),
        KeyCode::Digit0 => Some('0'),
        KeyCode::Digit1 => Some('1'),
        KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'),
        KeyCode::Digit4 => Some('4'),
        KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'),
        KeyCode::Digit7 => Some('7'),
        KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some('-'),
        _ => None,
    }
}
