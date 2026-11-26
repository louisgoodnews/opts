//! The element index: which tile of the map holds the spline or object with a given ID code.
//!
//! Timetable entries name paths by the ID code of the element they belong to (`StnLinks.cfg`,
//! `.ttr`; `docs/src/formats/map/timetables.md`). Turning such an ID into lanes needs to know
//! where the element is placed, and a course can cross tiles that are not resident.
//!
//! The index therefore never loads a tile itself. The host hands it tiles it has already parsed
//! ([`ElementIndex::index_tile`]); tiles it has not indexed are listed, never read. A lookup that
//! misses says which tiles are still unindexed, so the caller can report that instead of treating
//! the element as absent. [`ElementCache`] keeps one index per session and starts it afresh when
//! the chrono state (which changes tiles and `TTData` by date) changes.

use std::collections::{BTreeSet, HashMap};

use opts_core::formats::map::MapTile;

/// A tile of the map grid, as in `tile_<x>_<y>.map`.
pub type TileKey = (i32, i32);

/// The kind of placed element an ID code belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// A `[spline]` or `[spline_h]` record (its `.sli` profile carries the paths).
    Spline,
    /// An `[object]` record (its `.sco` carries the paths).
    Object,
}

/// Where an element is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementRef {
    /// The tile that holds the placement.
    pub tile: TileKey,
    /// Spline or object.
    pub kind: ElementKind,
}

/// The answer to a lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// The element is placed on an indexed tile.
    Found(ElementRef),
    /// Not on any indexed tile, and every tile the caller knows of is indexed.
    Absent,
    /// Not on any indexed tile, but these tiles are not indexed yet: the element may be on one of
    /// them. Nothing was loaded to find out.
    NotIndexed {
        /// The tiles the caller named that are not indexed (sorted).
        unindexed_tiles: Vec<TileKey>,
    },
}

/// Map-wide `ID code -> (tile, kind)` for the tiles indexed so far.
#[derive(Debug, Clone, Default)]
pub struct ElementIndex {
    by_id: HashMap<u64, ElementRef>,
    tiles: BTreeSet<TileKey>,
    /// Later placements of an ID code that is already indexed: `(id, where it was found again)`.
    /// The first placement stays the answer; the list is kept so a duplicate is not silent.
    duplicates: Vec<(u64, ElementRef)>,
}

impl ElementIndex {
    /// An empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `tile` has been indexed.
    #[must_use]
    pub fn is_indexed(&self, tile: TileKey) -> bool {
        self.tiles.contains(&tile)
    }

    /// Number of indexed tiles.
    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Number of indexed element ID codes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no element is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Duplicate placements found while indexing (the first placement wins).
    #[must_use]
    pub fn duplicates(&self) -> &[(u64, ElementRef)] {
        &self.duplicates
    }

    /// Indexes the splines and objects of an already parsed `tile`. Indexing the same tile twice
    /// does nothing. Returns the number of ID codes added.
    pub fn index_tile(&mut self, tile: TileKey, parsed: &MapTile) -> usize {
        if !self.tiles.insert(tile) {
            return 0;
        }
        let splines = parsed.splines.iter().map(|s| (s.id_code, ElementKind::Spline));
        let objects = parsed.objects.iter().map(|o| (o.id_code, ElementKind::Object));
        let mut added = 0;
        for (id, kind) in splines.chain(objects) {
            let found = ElementRef { tile, kind };
            match self.by_id.get(&id) {
                None => {
                    self.by_id.insert(id, found);
                    added += 1;
                }
                Some(first) if *first != found => self.duplicates.push((id, found)),
                // The same element on the same tile twice (a repeated record): nothing new.
                Some(_) => {}
            }
        }
        added
    }

    /// Looks up `id`. `known_tiles` are the tiles the map has (for example from `global.cfg`);
    /// a miss is [`Lookup::NotIndexed`] when any of them is still unindexed.
    #[must_use]
    pub fn lookup(&self, id: u64, known_tiles: &[TileKey]) -> Lookup {
        if let Some(found) = self.by_id.get(&id) {
            return Lookup::Found(*found);
        }
        let mut unindexed_tiles: Vec<TileKey> = known_tiles
            .iter()
            .copied()
            .filter(|t| !self.tiles.contains(t))
            .collect();
        if unindexed_tiles.is_empty() {
            Lookup::Absent
        } else {
            unindexed_tiles.sort_unstable();
            unindexed_tiles.dedup();
            Lookup::NotIndexed { unindexed_tiles }
        }
    }

    /// The tiles of `known_tiles` that are not indexed yet, in name order: what to parse next.
    #[must_use]
    pub fn missing(&self, known_tiles: &[TileKey]) -> Vec<TileKey> {
        let mut out: Vec<TileKey> = known_tiles
            .iter()
            .copied()
            .filter(|t| !self.tiles.contains(t))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// The session's element index, keyed by the chrono state it was built from.
///
/// Calling [`ElementCache::get`] with a different `state_key` (a hash of the active chrono and
/// `TTData` selection, chosen by the host) discards the index, because the tiles and timetables
/// of another chrono state may place the same ID code elsewhere.
#[derive(Debug, Default)]
pub struct ElementCache {
    state_key: Option<u64>,
    index: ElementIndex,
}

impl ElementCache {
    /// The index for `state_key`, emptied first when it differs from the last one.
    pub fn get(&mut self, state_key: u64) -> &mut ElementIndex {
        if self.state_key != Some(state_key) {
            self.state_key = Some(state_key);
            self.index = ElementIndex::new();
        }
        &mut self.index
    }

    /// Drops the index (the next [`Self::get`] starts empty).
    pub fn clear(&mut self) {
        self.state_key = None;
        self.index = ElementIndex::new();
    }
}
