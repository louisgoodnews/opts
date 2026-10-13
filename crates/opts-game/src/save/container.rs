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

/// Why a save could not be read, written or found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The file system said no.
    Io(String),
    /// The file is not valid JSON, or not shaped like a save.
    Corrupt(String),
    /// Valid JSON that is not an OPTS save (wrong or missing `magic`).
    NotASave,
    /// Written by a newer build.
    NewerFormat {
        /// The file's `format_version`.
        found: u32,
        /// The newest version this build reads.
        supported: u32,
    },
    /// The file or a section is bigger than the limit.
    TooLarge(String),
    /// No save with this id.
    NotFound(String),
    /// The name or id is not acceptable.
    InvalidName(String),
    /// A save with this name exists already (its id).
    Exists(String),
    /// There is no running session to save.
    NoSession(String),
    /// A section's capture function failed.
    Capture {
        /// Section id.
        section: String,
        /// What went wrong.
        message: String,
    },
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(m) => write!(f, "file error: {m}"),
            Self::Corrupt(m) => write!(f, "the save file is damaged: {m}"),
            Self::NotASave => write!(f, "the file is not an OPTS save"),
            Self::NewerFormat { found, supported } => write!(
                f,
                "the save was written by a newer version (format {found}, this build reads up to {supported})"
            ),
            Self::TooLarge(m) => write!(f, "too large: {m}"),
            Self::NotFound(id) => write!(f, "no save `{id}`"),
            Self::InvalidName(m) => write!(f, "invalid name: {m}"),
            Self::Exists(id) => write!(f, "a save called like this exists already (`{id}`)"),
            Self::NoSession(m) => write!(f, "{m}"),
            Self::Capture { section, message } => {
                write!(f, "section `{section}` could not be captured: {message}")
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl SaveError {
    /// A stable word for logs, the control protocol and the UI's message choice.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Corrupt(_) => "corrupt",
            Self::NotASave => "not_a_save",
            Self::NewerFormat { .. } => "newer_format",
            Self::TooLarge(_) => "too_large",
            Self::NotFound(_) => "not_found",
            Self::InvalidName(_) => "invalid_name",
            Self::Exists(_) => "exists",
            Self::NoSession(_) => "no_session",
            Self::Capture { .. } => "capture",
        }
    }
}

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
        let text = serde_json::to_string_pretty(self).map_err(|e| SaveError::Corrupt(e.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> SaveFile {
        let mut f = SaveFile::new(SaveMeta {
            name: "Test".into(),
            map: "Berlin".into(),
            vehicles: vec!["Bus".into()],
            play_time_s: 12.5,
            ..Default::default()
        });
        f.sections.insert(
            "x".into(),
            SectionBlob {
                version: 3,
                data: json!({"a": 1}),
            },
        );
        f
    }

    #[test]
    fn a_file_round_trips() {
        let f = sample();
        assert_eq!(SaveFile::decode(&f.encode().expect("encode")).expect("decode"), f);
    }

    #[test]
    fn a_newer_format_is_refused_even_when_the_layout_changed() {
        let text = json!({"magic": MAGIC, "format_version": 99, "something": "else"}).to_string();
        assert_eq!(
            SaveFile::decode(&text),
            Err(SaveError::NewerFormat {
                found: 99,
                supported: FORMAT_VERSION
            })
        );
    }

    #[test]
    fn other_json_and_garbage_are_told_apart() {
        assert_eq!(SaveFile::decode("[1,2]"), Err(SaveError::NotASave));
        assert_eq!(SaveFile::decode("{\"a\":1}"), Err(SaveError::NotASave));
        assert!(matches!(SaveFile::decode("{\"magic\": \"OPTS-"), Err(SaveError::Corrupt(_))));
        assert!(matches!(
            SaveFile::decode(&json!({"magic": MAGIC}).to_string()),
            Err(SaveError::Corrupt(_))
        ));
    }

    #[test]
    fn oversized_sections_are_refused_on_write() {
        let mut f = sample();
        f.sections.insert(
            "big".into(),
            SectionBlob {
                version: 1,
                data: json!("x".repeat(MAX_SECTION_BYTES + 1)),
            },
        );
        assert!(matches!(f.encode(), Err(SaveError::TooLarge(_))));
    }

    #[test]
    fn names_are_cleaned_and_slugged() {
        assert_eq!(clean_name("  Fahrt\u{0}\n 1 ").as_deref(), Ok("Fahrt 1"));
        assert!(clean_name("   ").is_err());
        assert_eq!(clean_name(&"a".repeat(100)).map(|n| n.chars().count()), Ok(MAX_NAME_CHARS));
        assert_eq!(slug("Meine Fahrt, Nr. 1!"), "meine-fahrt-nr-1");
        assert_eq!(slug("???"), "save");
        assert!(valid_id(&slug("../../etc/passwd")));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }
}
