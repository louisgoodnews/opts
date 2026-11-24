//! Chrono entries: `Chrono.cfg`, the folder scan, activation by date and the application order.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::data::merge_ailists;
use crate::formats::text::decode_bytes;

/// One Chrono entry: a folder below `Chrono/` with a `Chrono.cfg`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChronoEntry {
    /// The entry folder.
    pub dir: PathBuf,
    /// Path of the folder below `Chrono/`, `/`-separated (`0200_Grenzoeffnung`,
    /// `1_FCA Heimspiele/02.11.25 - 14.00 Uhr`).
    pub rel: String,
    /// `[startdate]`: first active day (`None`: from the beginning).
    pub start: Option<ChronoDate>,
    /// `[enddate]`: last active day, inclusive (`None`: open-ended).
    pub end: Option<ChronoDate>,
    /// `[name]` of `Chrono.cfg`.
    pub name: Option<String>,
    /// `[description]` … `[end]` of `Chrono.cfg`.
    pub description: Option<String>,
    /// `[name]` of each `Chrono_<LANG>.dsc`: `(LANG upper-case, name)`.
    pub localized_names: Vec<(String, String)>,
    /// `[deactivate_lines]`: timetable lines switched off while the entry is active.
    pub deactivate_lines: Vec<String>,
    /// `[ticketpack]` (`.otp` paths; the keyword may repeat).
    pub ticketpacks: Vec<String>,
    /// `[moneysystem]` (`Money\…\….cfg`).
    pub moneysystem: Option<String>,
    /// Overlay tiles (`tile_X_Y.map`) by tile key.
    pub tiles: BTreeMap<(i32, i32), PathBuf>,
    /// Tiles marked `<tile>.map.chronoterrrelevant`.
    pub terrain_relevant: BTreeSet<(i32, i32)>,
    /// Other files directly in the folder (data files), by lower-case name.
    pub files: BTreeMap<String, PathBuf>,
    /// Files of the entry's `TTData/` folder, by lower-case name.
    pub ttdata: BTreeMap<String, PathBuf>,
}

impl ChronoEntry {
    /// Whether the entry is active on `date` (`start ≤ date ≤ end`).
    #[must_use]
    pub fn is_active(&self, date: ChronoDate) -> bool {
        self.start.is_none_or(|s| s <= date) && self.end.is_none_or(|e| date <= e)
    }

    /// A name to show: the `.dsc` name in `lang` (`ENG`, `DEU`, …), else `[name]`, else an English
    /// or German `.dsc` name, else the folder name.
    #[must_use]
    pub fn display_name(&self, lang: &str) -> String {
        let localized = |l: &str| {
            self.localized_names
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(l))
                .map(|(_, n)| n.clone())
        };
        localized(lang)
            .or_else(|| self.name.clone())
            .or_else(|| localized("ENG"))
            .or_else(|| localized("DEU"))
            .or_else(|| self.localized_names.first().map(|(_, n)| n.clone()))
            .unwrap_or_else(|| self.rel.rsplit('/').next().unwrap_or_default().to_string())
    }

    /// The key entries are applied in: start date (missing = −∞), then the folder path.
    pub(super) fn order_key(&self) -> (u32, String) {
        (
            self.start.map_or(0, ChronoDate::yyyymmdd),
            self.rel.to_lowercase(),
        )
    }
}

/// The tile key of `tile_X_Y.map` (case-insensitive).
#[must_use]
pub(super) fn tile_key_of(name: &str) -> Option<(i32, i32)> {
    let lower = name.to_ascii_lowercase();
    let core = lower.strip_prefix("tile_")?.strip_suffix(".map")?;
    let (x, y) = core.split_once('_')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// The child of `dir` called `name`, compared case-insensitively.
pub(super) fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

mod cfg;
mod date;
mod derive;
mod outputs;
mod scan;
mod set;

pub use date::ChronoDate;
pub use outputs::*;
use scan::{files_of, walk};
pub use set::*;

#[cfg(test)]
mod tests;
