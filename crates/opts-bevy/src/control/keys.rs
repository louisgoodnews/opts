//! Canonical key names of the control protocol and their Bevy equivalents.

use bevy::input::keyboard::{Key, KeyCode};
use smol_str::SmolStr;

mod parse;
mod tables;

pub use parse::*;
use tables::*;

/// The canonical protocol name of a physical key (the inverse of [`parse_key`]), if it has one.
#[must_use]
pub fn key_name(code: KeyCode) -> Option<String> {
    if let Some(i) = LETTERS.iter().position(|k| *k == code) {
        return Some(((b'A' + i as u8) as char).to_string());
    }
    if let Some(i) = DIGITS.iter().position(|k| *k == code) {
        return Some(i.to_string());
    }
    if let Some(i) = FUNCTION.iter().position(|(k, _)| *k == code) {
        return Some(format!("F{}", i + 1));
    }
    NAMED
        .iter()
        .find(|n| named(n).is_some_and(|(k, _)| k == code))
        .map(|n| (*n).to_string())
}

/// Every accepted name pattern, for error messages.
#[must_use]
pub fn key_names() -> Vec<String> {
    let mut v: Vec<String> = vec!["A-Z".into(), "0-9".into(), "F1-F12".into()];
    v.extend(NAMED.iter().map(|s| (*s).to_string()));
    v
}

#[cfg(test)]
mod tests;
