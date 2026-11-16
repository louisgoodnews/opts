//! The keys of viewpoints: the key of a bus file, its model name, and the normalised key the file stores.

use super::*;

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
