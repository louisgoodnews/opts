//! Train formations: an ordered list of cars with an orientation each.
//!
//! The model is the one OMSI's `.zug` consist files use (`docs/src/formats/text/zug.md`): a list of
//! `(vehicle, orientation)` pairs. It is pure data: resolving a vehicle to a file, validating the
//! formation against an installation and spawning it happen in `opts-game`.
//! Design: `docs/src/architecture/train-formation-editor.md`, ADR-0013.

mod zug;

use serde::{Deserialize, Serialize};

pub use zug::{ZugError, ZugNote, MAX_CARS};

/// A train: cars front to back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainFormation {
    /// Display name (the file name for a `.zug`, the user's name for a custom formation).
    pub name: String,
    /// The cars, front to back.
    pub segments: Vec<FormationSegment>,
    /// Where the formation comes from.
    pub source: FormationSource,
}

/// One car of a formation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormationSegment {
    /// The vehicle.
    pub vehicle: VehicleRef,
    /// Whether the car runs forward or turned around.
    pub orientation: Orientation,
}

/// What a `.zug` entry names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VehicleRef {
    /// A vehicle file relative to the OMSI root, as written (backslashes, any case).
    File(String),
    /// A vehicle group name that the map resolves (`M-Train Cars`).
    Group(String),
}

impl VehicleRef {
    /// The file path or the group name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::File(s) | Self::Group(s) => s,
        }
    }

    /// The reference of one `.zug` line: a path or file name with an `.ovh` or `.bus` extension, or
    /// anything containing a path separator, is a file; a bare name is a group.
    #[must_use]
    pub fn from_entry(text: &str) -> Self {
        let t = text.trim();
        let lower = t.to_ascii_lowercase();
        if lower.ends_with(".ovh")
            || lower.ends_with(".bus")
            || lower.contains('\\')
            || lower.contains('/')
        {
            Self::File(t.to_owned())
        } else {
            Self::Group(t.to_owned())
        }
    }
}

/// Orientation of a car in the train (`.zug`: `0` / `1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    /// The car's front faces the front of the train.
    #[default]
    Forward,
    /// The car is turned around.
    Backward,
}

impl Orientation {
    /// The `.zug` value.
    #[must_use]
    pub const fn zug_value(self) -> u8 {
        match self {
            Self::Forward => 0,
            Self::Backward => 1,
        }
    }

    /// The other one.
    #[must_use]
    pub const fn flipped(self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }
}

/// Where a formation comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormationSource {
    /// Composed by the user.
    Custom,
    /// Read from this `.zug` (path relative to the OMSI root).
    ZugFile(String),
}

impl TrainFormation {
    /// An empty custom formation.
    #[must_use]
    pub fn custom(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            segments: Vec::new(),
            source: FormationSource::Custom,
        }
    }

    /// Number of cars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Whether the formation has no cars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// The vehicle files named (groups left out), in order, with repeats.
    pub fn file_refs(&self) -> impl Iterator<Item = &str> {
        self.segments.iter().filter_map(|s| match &s.vehicle {
            VehicleRef::File(f) => Some(f.as_str()),
            VehicleRef::Group(_) => None,
        })
    }

    /// Whether any car is a group name.
    #[must_use]
    pub fn has_groups(&self) -> bool {
        self.segments
            .iter()
            .any(|s| matches!(s.vehicle, VehicleRef::Group(_)))
    }
}

#[cfg(test)]
mod tests;
