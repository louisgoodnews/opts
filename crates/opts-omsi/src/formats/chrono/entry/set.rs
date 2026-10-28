//! Active sets, tile overlay plans, summaries, switches and `ChronoSet` resolution.

use super::*;

/// Indices of the active entries of a [`ChronoSet`], in application order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActiveSet(pub(super) Vec<usize>);

impl ActiveSet {
    /// Entry indices in application order.
    #[must_use]
    pub fn indices(&self) -> &[usize] {
        &self.0
    }

    /// Whether entry `i` is active.
    #[must_use]
    pub fn contains(&self, i: usize) -> bool {
        self.0.contains(&i)
    }

    /// Whether no entry is active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Which overlays apply to which tile for one [`ActiveSet`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChronoPlan {
    /// Overlay tile files per tile, in application order.
    pub tiles: BTreeMap<(i32, i32), Vec<PathBuf>>,
    /// Tiles with a `.chronoterrrelevant` marker in an active entry.
    pub terrain_relevant: BTreeSet<(i32, i32)>,
}

impl ChronoPlan {
    /// The overlays of a tile, in application order (empty for an untouched tile).
    #[must_use]
    pub fn overlays(&self, key: (i32, i32)) -> &[PathBuf] {
        self.tiles.get(&key).map_or(&[], Vec::as_slice)
    }
}

/// What the active entries set besides tiles and data files (exposed; no consumer yet).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChronoSummary {
    /// `[deactivate_lines]` of all active entries (in order, without duplicates).
    pub deactivate_lines: Vec<String>,
    /// `[ticketpack]` of the last active entry that names one.
    pub ticketpacks: Vec<String>,
    /// `[moneysystem]` of the last active entry that names one.
    pub moneysystem: Option<String>,
}

/// The difference between two active sets: what a live switch has to rebuild.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChronoSwitch {
    /// Entries active in exactly one of the two sets.
    pub toggled: Vec<usize>,
    /// Tiles whose overlay list differs.
    pub tiles: BTreeSet<(i32, i32)>,
    /// The 3 × 3 neighbourhoods of tiles marked `.chronoterrrelevant` in a toggled entry: their
    /// terrain carve has to be recomputed.
    pub carve_ring: BTreeSet<(i32, i32)>,
    /// Lower-case names of the data files in toggled entries (`unsched_trafficdens.txt`,
    /// `ailists_#upd.cfg`, `parklist_p.txt`, …).
    pub data_files: BTreeSet<String>,
    /// Whether a toggled entry has a `TTData/` folder.
    pub ttdata: bool,
}

impl ChronoSwitch {
    /// Whether nothing changes.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.toggled.is_empty()
    }

    /// Whether a data file called `name` (case-insensitive) changes.
    #[must_use]
    pub fn touches(&self, name: &str) -> bool {
        self.data_files.contains(&name.to_lowercase())
    }

    /// Whether a data file whose name starts with `prefix` (case-insensitive) changes.
    #[must_use]
    pub fn touches_prefix(&self, prefix: &str) -> bool {
        let p = prefix.to_lowercase();
        self.data_files.iter().any(|f| f.starts_with(&p))
    }

    /// Tiles to rebuild: the changed tiles and the carve ring.
    #[must_use]
    pub fn rebuild_tiles(&self) -> BTreeSet<(i32, i32)> {
        self.tiles.union(&self.carve_ring).copied().collect()
    }
}

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

    fn active_entries<'a>(
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

    /// The timetable files in force: the map's `TTData/` overlaid file by file by the active
    /// entries' `TTData/` (by lower-case name).
    #[must_use]
    pub fn ttdata(&self, active: &ActiveSet) -> BTreeMap<String, PathBuf> {
        let mut out = BTreeMap::new();
        if let Some(dir) = child_ci(&self.map_dir, "TTData").filter(|d| d.is_dir()) {
            out.extend(files_of(&dir));
        }
        for e in self.active_entries(active) {
            out.extend(e.ttdata.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        out
    }

    /// The text of `ailists.cfg` in force: the map's file (the only place a map's list is read
    /// from; an active entry's own `ailists.cfg` replaces it) with the `ailists_#upd.cfg` of every
    /// active entry merged in order ([`merge_ailists`]). `None` when the map has no `ailists.cfg`
    /// and no active entry updates it.
    #[must_use]
    pub fn ailists(&self, active: &ActiveSet) -> Option<String> {
        let mut text = child_ci(&self.map_dir, "ailists.cfg")
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| decode_bytes(&b));
        for e in self.active_entries(active) {
            // A complete list in the entry's own folder replaces the map's.
            if let Some(full) = e
                .files
                .get("ailists.cfg")
                .and_then(|p| std::fs::read(p).ok())
            {
                text = Some(decode_bytes(&full));
            }
            let Some(update) = e
                .files
                .get("ailists_#upd.cfg")
                .and_then(|p| std::fs::read(p).ok())
            else {
                continue;
            };
            let base = text.take().unwrap_or_default();
            text = Some(merge_ailists(&base, &decode_bytes(&update)));
        }
        text
    }

    /// Deactivated lines, ticket pack and money system of an active set.
    #[must_use]
    pub fn summary(&self, active: &ActiveSet) -> ChronoSummary {
        let mut out = ChronoSummary::default();
        for e in self.active_entries(active) {
            for l in &e.deactivate_lines {
                if !out
                    .deactivate_lines
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(l))
                {
                    out.deactivate_lines.push(l.clone());
                }
            }
            if !e.ticketpacks.is_empty() {
                out.ticketpacks.clone_from(&e.ticketpacks);
            }
            if e.moneysystem.is_some() {
                out.moneysystem.clone_from(&e.moneysystem);
            }
        }
        out
    }

    /// What changes between two active sets.
    #[must_use]
    pub fn switch(&self, old: &ActiveSet, new: &ActiveSet) -> ChronoSwitch {
        let mut out = ChronoSwitch::default();
        if old == new {
            return out;
        }
        out.toggled = (0..self.entries.len())
            .filter(|&i| old.contains(i) != new.contains(i))
            .collect();
        let (a, b) = (self.plan(old), self.plan(new));
        for key in a.tiles.keys().chain(b.tiles.keys()) {
            if a.overlays(*key) != b.overlays(*key) {
                out.tiles.insert(*key);
            }
        }
        for &i in &out.toggled {
            let e = &self.entries[i];
            for &(x, y) in &e.terrain_relevant {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        out.carve_ring.insert((x + dx, y + dy));
                    }
                }
            }
            out.data_files.extend(e.files.keys().cloned());
            out.ttdata |= !e.ttdata.is_empty();
        }
        out
    }
}
