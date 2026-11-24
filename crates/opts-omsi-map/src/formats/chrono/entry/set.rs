//! Chrono entry set of a map: scan, date activation, the tile plan and the effective files.

use super::*;

/// The Chrono entries of a map, in application order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChronoSet {
    pub(super) map_dir: PathBuf,
    pub(super) entries: Vec<ChronoEntry>,
}

impl ChronoSet {
    /// A set without entries (a map without `Chrono/`).
    #[must_use]
    pub fn empty(map_dir: &Path) -> Self {
        Self {
            map_dir: map_dir.to_path_buf(),
            entries: Vec::new(),
        }
    }

    /// Scans `<map_dir>/Chrono/` recursively (folder names compared case-insensitively). Unreadable
    /// folders and files are skipped.
    #[must_use]
    pub fn scan(map_dir: &Path) -> Self {
        let mut set = Self::empty(map_dir);
        if let Some(dir) = child_ci(map_dir, "Chrono").filter(|d| d.is_dir()) {
            walk(&dir, "", &mut set.entries);
        }
        set.entries.sort_by_cached_key(ChronoEntry::order_key);
        set
    }

    /// Builds a set from entries (sorted into application order).
    #[must_use]
    pub fn from_entries(map_dir: &Path, mut entries: Vec<ChronoEntry>) -> Self {
        entries.sort_by_cached_key(ChronoEntry::order_key);
        Self {
            map_dir: map_dir.to_path_buf(),
            entries,
        }
    }

    /// The map folder.
    #[must_use]
    pub fn map_dir(&self) -> &Path {
        &self.map_dir
    }

    /// The entries in application order.
    #[must_use]
    pub fn entries(&self) -> &[ChronoEntry] {
        &self.entries
    }

    /// The entries active on `date`.
    #[must_use]
    pub fn active(&self, date: ChronoDate) -> ActiveSet {
        ActiveSet(
            (0..self.entries.len())
                .filter(|&i| self.entries[i].is_active(date))
                .collect(),
        )
    }

    /// Every entry with its state on `date`, in application order.
    #[must_use]
    pub fn states(&self, date: ChronoDate) -> Vec<(&ChronoEntry, bool)> {
        self.entries
            .iter()
            .map(|e| (e, e.is_active(date)))
            .collect()
    }

    pub(super) fn active_entries<'a>(
        &'a self,
        active: &'a ActiveSet,
    ) -> impl DoubleEndedIterator<Item = &'a ChronoEntry> {
        active.0.iter().filter_map(|&i| self.entries.get(i))
    }

    /// The overlays per tile and the carve markers of an active set.
    #[must_use]
    pub fn plan(&self, active: &ActiveSet) -> ChronoPlan {
        let mut plan = ChronoPlan::default();
        for e in self.active_entries(active) {
            for (key, path) in &e.tiles {
                plan.tiles.entry(*key).or_default().push(path.clone());
            }
            plan.terrain_relevant
                .extend(e.terrain_relevant.iter().copied());
        }
        plan
    }

    /// The file a map-level data file is read from: that of the last active entry that has it
    /// (`parklist_p*.txt`, `registrations.txt`, `unsched_trafficdens.txt` replace the whole
    /// file), else the map's own (`None` when neither exists). Names compare case-insensitively.
    #[must_use]
    pub fn effective_file(&self, active: &ActiveSet, name: &str) -> Option<PathBuf> {
        let lower = name.to_lowercase();
        self.active_entries(active)
            .rev()
            .find_map(|e| e.files.get(&lower).cloned())
            .or_else(|| child_ci(&self.map_dir, name).filter(|p| p.is_file()))
    }
}
