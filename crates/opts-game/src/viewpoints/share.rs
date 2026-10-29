//! Sharing viewpoints (M-0084): the stored viewpoints are written to a file of their own, and such
//! a file is merged into the store. The exchange format is the `viewpoints.json` format itself,
//! so a shared file is also a valid store and a hand-edited one is read as carefully as the store.

use std::path::Path;

use super::{Section, ViewpointStore, Viewpoints};

/// What an import took from a file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportSummary {
    /// Vehicle entries (a vehicle and one of its cameras) taken from the file.
    pub vehicles: usize,
    /// Model entries (a model and one of its cameras) taken from the file.
    pub models: usize,
    /// Entries that replaced a different stored value.
    pub replaced: usize,
    /// What was dropped from the file: invalid entries, with the reason.
    pub problems: Vec<String>,
}

impl ViewpointStore {
    /// No vehicle and no model has an entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vehicles.is_empty() && self.models.is_empty()
    }

    /// Takes every entry of `other`. For the same vehicle (or model) and camera the entry of
    /// `other` wins; everything else stays.
    pub fn merge_from(&mut self, other: &Self) -> ImportSummary {
        let mut replaced = 0;
        let vehicles = take(&mut self.vehicles, &other.vehicles, &mut replaced);
        let models = take(&mut self.models, &other.models, &mut replaced);
        ImportSummary {
            vehicles,
            models,
            replaced,
            problems: Vec::new(),
        }
    }
}

fn take(into: &mut Section, from: &Section, replaced: &mut usize) -> usize {
    let mut taken = 0;
    for (key, cameras) in from {
        for (index, vp) in cameras {
            taken += 1;
            let before = into.entry(key.clone()).or_default().insert(*index, *vp);
            if before.is_some_and(|old| old != *vp) {
                *replaced += 1;
            }
        }
    }
    taken
}

/// Reads a file to import: the store it holds and the problems found in it. Unlike the store's own
/// file, a file that cannot be used is left where it is.
///
/// # Errors
/// A message when the file cannot be read, is not a viewpoints file or is of a newer format.
pub fn read_import(path: &Path) -> Result<(ViewpointStore, Vec<String>), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let (store, report) = ViewpointStore::parse(&text);
    if report.unreadable || (store.is_empty() && !report.problems.is_empty()) {
        return Err(format!(
            "{} is not a usable viewpoints file: {}",
            path.display(),
            report.problems.join("; ")
        ));
    }
    Ok((store, report.problems))
}

impl Viewpoints {
    /// Writes the store to `path` as a file to share.
    ///
    /// # Errors
    /// The I/O error when the file cannot be written.
    pub fn export_to(&self, path: &Path) -> std::io::Result<()> {
        self.store.save_to(path)
    }

    /// Merges `incoming` into the stored viewpoints and writes the file. Like a save it starts
    /// from what the file holds now, because another run may have written meanwhile.
    ///
    /// # Errors
    /// The I/O error when the file cannot be written; the in-memory store is updated anyway.
    pub fn import(&mut self, incoming: &ViewpointStore) -> std::io::Result<ImportSummary> {
        let mut store = self
            .path
            .as_deref()
            .map_or_else(|| self.store.clone(), ViewpointStore::load_from);
        let summary = store.merge_from(incoming);
        self.store = store;
        match &self.path {
            Some(p) => self.store.save_to(p).map(|()| summary),
            None => Ok(summary),
        }
    }
}

#[cfg(test)]
mod tests;
