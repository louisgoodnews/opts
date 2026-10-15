//! The user's saved formations: one TOML file each in `<user data dir>/formations/`.
//!
//! ```toml
//! format = 1
//! name = "Three NF6D"
//! template = "Trains\\X10_112_IC-8.zug"   # optional: the preset it started from
//!
//! [[segment]]
//! vehicle = "Vehicles\\Tram_NF6D\\NF6D_A.bus"
//! orientation = "forward"                  # forward | backward
//! powered = true                           # optional
//! ```
//!
//! A segment names a `vehicle` file or a vehicle `group`. Files are written atomically (a temporary
//! file renamed over the target) and a file that cannot be read is listed with its error, never
//! deleted. Schema and rules: `docs/src/architecture/train-formation-editor.md` section 8.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use directories::ProjectDirs;
use opts_core::formats::trains::{
    FormationSegment, FormationSource, Orientation, TrainFormation, VehicleRef, MAX_CARS,
};
use serde::{Deserialize, Serialize};

use crate::save::container::{slug, valid_id};

/// The schema version this build reads and writes.
pub const FORMAT: u32 = 1;
/// Files larger than this are refused.
const MAX_FILE_BYTES: u64 = 1 << 20;
const EXT: &str = "toml";

/// Why a formation file could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// The id is not a formation id.
    #[error("`{0}` is not a formation id")]
    InvalidName(String),
    /// No such formation.
    #[error("no saved formation `{0}`")]
    NotFound(String),
    /// A file system error.
    #[error("{0}")]
    Io(String),
    /// The text is not a formation file.
    #[error("{0}")]
    Corrupt(String),
    /// Written by a newer version of OPTS.
    #[error("written by a newer OPTS (format {0}, this build reads {FORMAT})")]
    Newer(u32),
}

#[derive(Debug, Serialize, Deserialize)]
struct FileV1 {
    format: u32,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template: Option<String>,
    #[serde(default, rename = "segment")]
    segments: Vec<SegmentV1>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SegmentV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vehicle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(default = "forward")]
    orientation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    powered: Option<bool>,
}

fn forward() -> String {
    "forward".to_owned()
}

/// The TOML text of a formation.
///
/// # Errors
/// [`StoreError::Corrupt`] when the text cannot be produced.
pub fn encode(f: &TrainFormation) -> Result<String, StoreError> {
    let file = FileV1 {
        format: FORMAT,
        name: f.name.clone(),
        template: match &f.source {
            FormationSource::ZugFile(p) => Some(p.clone()),
            FormationSource::Custom => None,
        },
        segments: f
            .segments
            .iter()
            .map(|s| SegmentV1 {
                vehicle: matches!(s.vehicle, VehicleRef::File(_))
                    .then(|| s.vehicle.as_str().to_owned()),
                group: matches!(s.vehicle, VehicleRef::Group(_))
                    .then(|| s.vehicle.as_str().to_owned()),
                orientation: match s.orientation {
                    Orientation::Forward => "forward".to_owned(),
                    Orientation::Backward => "backward".to_owned(),
                },
                powered: s.powered,
            })
            .collect(),
    };
    toml::to_string_pretty(&file).map_err(|e| StoreError::Corrupt(e.to_string()))
}

/// The formation a TOML text holds.
///
/// # Errors
/// [`StoreError`] for text that is not a formation file, one from a newer version, or one with
/// more than [`MAX_CARS`] cars.
pub fn decode(text: &str) -> Result<TrainFormation, StoreError> {
    let file: FileV1 = toml::from_str(text).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    if file.format > FORMAT {
        return Err(StoreError::Newer(file.format));
    }
    if file.format == 0 {
        return Err(StoreError::Corrupt("format must be 1".to_owned()));
    }
    if file.segments.len() > MAX_CARS {
        return Err(StoreError::Corrupt(format!("more than {MAX_CARS} cars")));
    }
    let mut segments = Vec::with_capacity(file.segments.len());
    for (i, s) in file.segments.iter().enumerate() {
        let vehicle = match (&s.vehicle, &s.group) {
            (Some(v), None) if !v.trim().is_empty() => VehicleRef::File(v.trim().to_owned()),
            (None, Some(g)) if !g.trim().is_empty() => VehicleRef::Group(g.trim().to_owned()),
            _ => {
                return Err(StoreError::Corrupt(format!(
                    "segment {}: give exactly one of `vehicle` and `group`",
                    i + 1
                )))
            }
        };
        let orientation = match s.orientation.to_ascii_lowercase().as_str() {
            "forward" => Orientation::Forward,
            "backward" => Orientation::Backward,
            other => {
                return Err(StoreError::Corrupt(format!(
                    "segment {}: orientation `{other}` is not forward or backward",
                    i + 1
                )))
            }
        };
        segments.push(FormationSegment {
            vehicle,
            orientation,
            powered: s.powered,
        });
    }
    Ok(TrainFormation {
        name: file.name,
        segments,
        source: file
            .template
            .map_or(FormationSource::Custom, FormationSource::ZugFile),
    })
}

/// One file of the store.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedFormation {
    /// File stem; what the control protocol and the UI refer to.
    pub id: String,
    /// The formation, or why the file cannot be used.
    pub state: Result<TrainFormation, StoreError>,
    /// When the file was last written.
    pub modified: Option<SystemTime>,
}

/// The folder of saved formations.
#[derive(Debug, Clone)]
pub struct FormationStore {
    dir: PathBuf,
}

impl FormationStore {
    /// A store in `dir` (created on the first write).
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `<user data dir>/formations`.
    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        ProjectDirs::from("org", "opts", "opts").map(|d| d.data_dir().join("formations"))
    }

    /// The user's store, if a data directory is known.
    #[must_use]
    pub fn user() -> Option<Self> {
        Self::default_dir().map(Self::new)
    }

    /// The folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXT}"))
    }

    fn check(id: &str) -> Result<(), StoreError> {
        if valid_id(id) {
            Ok(())
        } else {
            Err(StoreError::InvalidName(id.to_owned()))
        }
    }

    /// An id for a formation called `name` that no file uses yet (`name`, `name-2`, ...).
    #[must_use]
    pub fn unique_id(&self, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.path(&id).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    /// Writes `formation` as `id` (replacing that file), atomically.
    ///
    /// # Errors
    /// [`StoreError`] for a bad id or a file system failure.
    pub fn write(&self, id: &str, formation: &TrainFormation) -> Result<(), StoreError> {
        Self::check(id)?;
        let text = encode(formation)?;
        let io = |e: std::io::Error| StoreError::Io(format!("{}: {e}", self.dir.display()));
        fs::create_dir_all(&self.dir).map_err(io)?;
        let tmp = self.dir.join(format!("{id}.{EXT}.tmp"));
        fs::write(&tmp, text).map_err(io)?;
        fs::rename(&tmp, self.path(id)).map_err(io)
    }

    /// Saves `formation` under a new id made from its name, and returns the id.
    ///
    /// # Errors
    /// [`StoreError`] on a file system failure.
    pub fn save_new(&self, formation: &TrainFormation) -> Result<String, StoreError> {
        let id = self.unique_id(&formation.name);
        self.write(&id, formation)?;
        Ok(id)
    }

    /// Reads formation `id`.
    ///
    /// # Errors
    /// [`StoreError`] when it does not exist or cannot be read.
    pub fn read(&self, id: &str) -> Result<TrainFormation, StoreError> {
        Self::check(id)?;
        Self::read_file(&self.path(id), id)
    }

    fn read_file(path: &Path, id: &str) -> Result<TrainFormation, StoreError> {
        let meta = fs::metadata(path).map_err(|_| StoreError::NotFound(id.to_owned()))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(StoreError::Corrupt(format!(
                "{} is {} bytes (limit {MAX_FILE_BYTES})",
                path.display(),
                meta.len()
            )));
        }
        let bytes =
            fs::read(path).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| StoreError::Corrupt("not UTF-8 text".to_owned()))?;
        decode(&text)
    }

    /// Deletes formation `id`.
    ///
    /// # Errors
    /// [`StoreError`] when it does not exist or cannot be removed.
    pub fn delete(&self, id: &str) -> Result<(), StoreError> {
        Self::check(id)?;
        fs::remove_file(self.path(id)).map_err(|_| StoreError::NotFound(id.to_owned()))
    }

    /// Every file of the store, newest first (files that cannot be read are listed with their
    /// error).
    #[must_use]
    pub fn list(&self) -> Vec<SavedFormation> {
        let Ok(rd) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<SavedFormation> = rd
            .flatten()
            .filter_map(|e| {
                let path = e.path();
                if path.extension().is_none_or(|x| x != EXT) {
                    return None;
                }
                let id = path.file_stem()?.to_string_lossy().into_owned();
                if !valid_id(&id) {
                    return None;
                }
                Some(SavedFormation {
                    state: Self::read_file(&path, &id),
                    modified: e.metadata().ok().and_then(|m| m.modified().ok()),
                    id,
                })
            })
            .collect();
        out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.id.cmp(&b.id)));
        out
    }
}

#[cfg(test)]
mod tests;
