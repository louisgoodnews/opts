//! Result types of a [`ChronoSet`]: the active indices, the tile plan, the summary and the switch.

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
