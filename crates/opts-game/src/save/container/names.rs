//! Names and ids of saves: the cleaned display name, the slug and the check of an id.

use super::*;

/// Cleans a name the user typed: trimmed, control characters removed, at most
/// [`MAX_NAME_CHARS`] characters.
///
/// # Errors
/// The name is empty after cleaning.
pub fn clean_name(raw: &str) -> Result<String, SaveError> {
    let name: String = raw
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect();
    let name = name.trim().to_owned();
    if name.is_empty() {
        Err(SaveError::InvalidName("the name is empty".to_owned()))
    } else {
        Ok(name)
    }
}

/// A file-name-safe id for `name`: lower-case letters and digits, other runs become one `-`.
#[must_use]
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_owned();
    if out.is_empty() {
        "save".to_owned()
    } else {
        out.chars().take(48).collect()
    }
}

/// Whether `id` can be a file stem of a save (so an id from outside can never leave the folder).
#[must_use]
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}
