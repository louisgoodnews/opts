//! The keys of stored viewpoints: the catalogue identity and model of the player's vehicle (the
//! components), and the pure helpers that read them from a `.bus` path. The store and its plugin
//! stay in `opts-game` (`viewpoints`), which re-exports these.

use std::path::Path;

use bevy::prelude::*;

/// Longest vehicle key kept.
const MAX_KEY_LEN: usize = 128;

/// The catalogue identity of the player's vehicle, on its root entity.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ViewpointKey(pub String);

/// The model of the player's vehicle (the folder of its `.bus` file, lower case), on its root.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ViewpointModel(pub String);

/// The model key of a `.bus` file: its folder name in lower case.
#[must_use]
pub fn model_of_bus_file(path: &Path) -> Option<String> {
    normalise_key(&path.parent()?.file_name()?.to_string_lossy())
}

/// The key of a `.bus` file: its stem in lower case.
#[must_use]
pub fn key_of_bus_file(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    normalise_key(&stem)
}

/// Lower-cases and trims a vehicle key; `None` when it is empty, too long or has control characters.
#[must_use]
pub fn normalise_key(key: &str) -> Option<String> {
    let k = key.trim().to_lowercase();
    (!k.is_empty() && k.len() <= MAX_KEY_LEN && !k.chars().any(char::is_control)).then_some(k)
}
