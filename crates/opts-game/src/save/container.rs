//! The container of a session save: magic, versions, metadata, a content fingerprint and the
//! sections. It knows nothing about what a section holds (`docs/src/architecture/session-save.md`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// First field of every save file.
pub const MAGIC: &str = "OPTS-SAVE";
/// The container format this build writes and reads. A file with a larger number is refused.
pub const FORMAT_VERSION: u32 = 1;
/// Largest save file that is read or written, in bytes.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Largest single section that is written, in bytes of its JSON text.
pub const MAX_SECTION_BYTES: usize = 4 * 1024 * 1024;
/// Longest save name, in characters.
pub const MAX_NAME_CHARS: usize = 60;

use opts_save_error as error;
mod names;
pub use error::SaveError;
pub use names::{clean_name, slug, valid_id};

/// What the save list shows without reading any section.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SaveMeta {
    /// The name the user gave it.
    pub name: String,
    /// When the save was first written (unix seconds).
    pub created_unix: i64,
    /// When it was last written (unix seconds).
    pub saved_unix: i64,
    /// Time spent driving this session, across loads (seconds).
    pub play_time_s: f64,
    /// Display name of the map.
    pub map: String,
    /// Display names of the user vehicles that were in the world.
    pub vehicles: Vec<String>,
    /// Reserved for a screenshot (a PNG as base64); absent for now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

/// Hashes of the content the save refers to, so a load can tell that a map or vehicle changed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Fingerprint {
    /// The map's folder name and the hash of its `global.cfg` and tile list.
    #[serde(default)]
    pub map: Option<(String, String)>,
    /// Vehicle `.bus` file name to the hash of the file.
    #[serde(default)]
    pub vehicles: BTreeMap<String, String>,
}

/// One section as stored: its own version and opaque data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionBlob {
    /// The section's format version.
    pub version: u32,
    /// The section's content.
    pub data: Value,
}

/// A whole save file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    /// Always [`MAGIC`].
    pub magic: String,
    /// The container format ([`FORMAT_VERSION`]).
    pub format_version: u32,
    /// The OPTS version that wrote the file.
    pub opts_version: String,
    /// What the list shows.
    pub meta: SaveMeta,
    /// Content hashes.
    #[serde(default)]
    pub fingerprint: Fingerprint,
    /// The sections by id; unknown ones are kept as they are.
    #[serde(default)]
    pub sections: BTreeMap<String, SectionBlob>,
}

impl SaveFile {
    /// An empty save of this build.
    #[must_use]
    pub fn new(meta: SaveMeta) -> Self {
        Self {
            magic: MAGIC.to_owned(),
            format_version: FORMAT_VERSION,
            opts_version: env!("CARGO_PKG_VERSION").to_owned(),
            meta,
            fingerprint: Fingerprint::default(),
            sections: BTreeMap::new(),
        }
    }

    /// The file's text: pretty JSON.
    ///
    /// # Errors
    /// A section is larger than [`MAX_SECTION_BYTES`] or the whole text larger than [`MAX_FILE_BYTES`].
    pub fn encode(&self) -> Result<String, SaveError> {
        for (id, blob) in &self.sections {
            let size = blob.data.to_string().len();
            if size > MAX_SECTION_BYTES {
                return Err(SaveError::TooLarge(format!(
                    "section `{id}` is {size} bytes (limit {MAX_SECTION_BYTES})"
                )));
            }
        }
        let text =
            serde_json::to_string_pretty(self).map_err(|e| SaveError::Corrupt(e.to_string()))?;
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err(SaveError::TooLarge(format!(
                "the save is {} bytes (limit {MAX_FILE_BYTES})",
                text.len()
            )));
        }
        Ok(text)
    }

    /// Reads a save from its text. The magic and the format version are checked first, so a file
    /// from a newer build is reported as such even when its layout has changed.
    ///
    /// # Errors
    /// See [`SaveError`].
    pub fn decode(text: &str) -> Result<Self, SaveError> {
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err(SaveError::TooLarge(format!(
                "the file is {} bytes (limit {MAX_FILE_BYTES})",
                text.len()
            )));
        }
        let value: Value =
            serde_json::from_str(text).map_err(|e| SaveError::Corrupt(e.to_string()))?;
        let Some(object) = value.as_object() else {
            return Err(SaveError::NotASave);
        };
        if object.get("magic").and_then(Value::as_str) != Some(MAGIC) {
            return Err(SaveError::NotASave);
        }
        let found = object
            .get("format_version")
            .and_then(Value::as_u64)
            .ok_or_else(|| SaveError::Corrupt("`format_version` is missing".to_owned()))?;
        let found = u32::try_from(found).unwrap_or(u32::MAX);
        if found > FORMAT_VERSION {
            return Err(SaveError::NewerFormat {
                found,
                supported: FORMAT_VERSION,
            });
        }
        serde_json::from_value(value).map_err(|e| SaveError::Corrupt(e.to_string()))
    }
}

#[cfg(test)]
mod tests;
