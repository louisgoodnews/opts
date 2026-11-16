//! Scan status, phases, counts and staleness verdicts.

/// Why (or whether) the catalogue needs a rescan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    /// No completed scan exists.
    Missing,
    /// The catalogue was built for a different OMSI root.
    RootChanged,
    /// The last scan is older than the allowed age.
    Expired {
        /// Age of the last scan.
        age_secs: i64,
    },
    /// The catalogue is current.
    Fresh {
        /// Age of the last scan.
        age_secs: i64,
    },
}

impl Staleness {
    /// Whether a rescan is required.
    #[must_use]
    pub fn needs_scan(self) -> bool {
        !matches!(self, Staleness::Fresh { .. })
    }
}

/// Scan phases reported to a progress callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Walking the filesystem.
    Walking,
    /// Detecting entities and reading their metadata.
    Detecting,
    /// Writing to SQLite.
    Writing,
}

/// Counts from a finished scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Files recorded.
    pub files: usize,
    /// Directories recorded.
    pub directories: usize,
    /// Entities recorded.
    pub entities: usize,
    /// References recorded.
    pub references: usize,
    /// References whose target does not exist anywhere in the installation.
    pub unresolved: usize,
    /// References whose path was wrong but whose file name exists elsewhere (rerouted).
    pub rerouted: usize,
}

/// The catalogue by kind (see [`crate::Catalogue::kind_counts`]); `other` is everything that is not one
/// of the four named kinds (trailers and AI vehicles, fonts, humans, weather, ...).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KindCounts {
    /// Files counted by the last scan.
    pub files: i64,
    /// Entries of every kind.
    pub entities: i64,
    /// Maps (as listed on Home and in the wizard).
    pub maps: i64,
    /// Player-selectable vehicles (as listed on Home and in the wizard).
    pub vehicles: i64,
    /// Scenery objects (`.sco`).
    pub scenery_objects: i64,
    /// Splines (`.sli`).
    pub splines: i64,
    /// All other entries.
    pub other: i64,
}
