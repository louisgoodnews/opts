//! Chrono in the running world (`docs/src/formats/map/chrono.md`,
//! `docs/src/architecture/session-environment.md` §5).
//!
//! The world job reads every tile through [`WorldLoadJob::read_tile`], which applies the
//! overlays of the entries active on the session's map date in order; overlay tiles without a
//! base file are added to the tile set. Data files (`parklist_p*.txt`, `registrations.txt`,
//! `unsched_trafficdens.txt`, `ailists.cfg` with its group updates) are read through
//! [`WorldLoadJob::data_file`] / [`WorldLoadJob::data_bytes`].
//!
//! A new map date while driving ([`MapDateChanged`]) is the live switch: compare the active sets;
//! if they differ, rebuild the resident tiles whose overlay list changed plus the 3 × 3
//! neighbourhoods of `.chronoterrrelevant` tiles (their terrain carve), the player's tile first,
//! and report the progress in [`ChronoRebuild`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::{CalendarDate, ChronoRebuild, MapDateChanged};
use opts_core::formats::chrono::{ActiveSet, ChronoDate, ChronoPlan, ChronoSet, TileOverlay};
use opts_core::formats::map::MapTile;

use super::reload::TileUnload;
use super::{TrafficPool, WorldLoadJob, WorldStats};

/// The Chrono state of the job: the scanned entries and what is active on the map date.
#[derive(Debug, Clone, Default)]
pub(super) struct ChronoContext {
    set: Arc<ChronoSet>,
    /// The map date (`None`: no date, the base map only).
    date: Option<ChronoDate>,
    active: ActiveSet,
    plan: ChronoPlan,
    /// Tiles that exist only as overlays (added to `tile_files`), with their first overlay.
    new_tiles: BTreeMap<(i32, i32), PathBuf>,
    /// Tiles of a running live switch that are not rebuilt yet.
    rebuilding: Vec<(i32, i32)>,
}

/// The date a session calendar date stands for in Chrono.
#[must_use]
pub fn chrono_date(d: CalendarDate) -> ChronoDate {
    ChronoDate::from_ymd(d.year, d.month, d.day)
}

/// What the active Chrono entries set besides tiles (deactivated lines, ticket pack, money
/// system). Exposed for later consumers (timetables, ticket sales); nothing reads it yet.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ChronoState {
    /// The map date.
    pub date: Option<ChronoDate>,
    /// Folders (below `Chrono/`) of the active entries, in application order.
    pub active: Vec<String>,
    /// Display names of the active entries.
    pub names: Vec<String>,
    /// Timetable lines the active entries switch off.
    pub deactivate_lines: Vec<String>,
    /// Ticket pack(s) in force.
    pub ticketpacks: Vec<String>,
    /// Money system in force.
    pub moneysystem: Option<String>,
}

/// What a live switch did.
#[derive(Debug, Default)]
pub(super) struct SwitchOutcome {
    /// Resident tiles queued for rebuilding, the player's tile first.
    pub(super) rebuild: Vec<(i32, i32)>,
    /// Tiles taken down for the rebuild.
    pub(super) unloads: Vec<TileUnload>,
    /// The AI lists or the unscheduled density changed: the traffic pool is rebuilt.
    pub(super) traffic: bool,
}

/// The order in which the tiles of a switch are rebuilt: resident ones only, the player's tile
/// first, then by Chebyshev distance from it, then by key.
pub(super) fn rebuild_order(
    tiles: &BTreeSet<(i32, i32)>,
    resident: impl Fn((i32, i32)) -> bool,
    player: Option<(i32, i32)>,
) -> Vec<(i32, i32)> {
    let mut out: Vec<(i32, i32)> = tiles.iter().copied().filter(|k| resident(*k)).collect();
    let dist = |k: (i32, i32)| player.map_or(0, |p| (k.0 - p.0).abs().max((k.1 - p.1).abs()));
    out.sort_by_key(|k| (dist(*k), *k));
    out
}

impl WorldLoadJob {
    /// Scans the map's `Chrono/` folder and sets the map date (`None`: the base map only).
    pub(super) fn init_chrono(&mut self, date: Option<ChronoDate>) {
        let set = ChronoSet::scan(&self.config.map_dir);
        if !set.entries().is_empty() {
            tracing::info!(entries = set.entries().len(), date = ?date.map(|d| d.to_string()), "Chrono entries found");
        }
        self.chrono.set = Arc::new(set);
        self.set_chrono_date(date);
    }

    /// Sets the map date: the active entries, the overlay plan and the tiles that exist only as
    /// overlays.
    fn set_chrono_date(&mut self, date: Option<ChronoDate>) {
        let c = &mut self.chrono;
        c.date = date;
        c.active = date.map_or_else(ActiveSet::default, |d| c.set.active(d));
        c.plan = c.set.plan(&c.active);
        for key in std::mem::take(&mut c.new_tiles).into_keys() {
            self.tile_files.remove(&key);
        }
        let mut new_tiles = BTreeMap::new();
        for (key, overlays) in &self.chrono.plan.tiles {
            if !self.tile_files.contains_key(key) {
                if let Some(first) = overlays.first() {
                    new_tiles.insert(*key, first.clone());
                }
            }
        }
        for (key, path) in &new_tiles {
            let name = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            self.tile_files.insert(*key, name);
        }
        if !new_tiles.is_empty() {
            tracing::info!(tiles = ?new_tiles.keys().collect::<Vec<_>>(), "Chrono adds tiles");
        }
        self.chrono.new_tiles = new_tiles;
        for (i, e) in self.chrono.set.entries().iter().enumerate() {
            if self.chrono.active.contains(i) {
                tracing::info!(entry = %e.rel, name = %e.display_name("ENG"), tiles = e.tiles.len(), "Chrono entry active");
            }
        }
    }

    /// The overlay files of a tile, in application order.
    pub(super) fn chrono_overlays(&self, key: (i32, i32)) -> &[PathBuf] {
        self.chrono.plan.overlays(key)
    }

    /// The overlay a tile without a base file comes from.
    pub(super) fn chrono_new_tile(&self, key: (i32, i32)) -> Option<PathBuf> {
        self.chrono.new_tiles.get(&key).cloned()
    }

    /// Applies the active overlays of `key` to its base tile; what could not be applied goes to
    /// the tile's `warnings` (the loader logs them).
    pub(super) fn apply_chrono(&self, key: (i32, i32), tile: &mut MapTile) {
        for path in self.chrono_overlays(key) {
            let overlay = match std::fs::read(path) {
                Ok(bytes) => TileOverlay::parse(&bytes),
                Err(e) => {
                    tile.warnings
                        .push(format!("Chrono overlay {} unreadable: {e}", path.display()));
                    continue;
                }
            };
            if tile.version.is_none() {
                tile.version = overlay.version;
            }
            let report = tile.apply_overlay(&overlay);
            let name = path
                .parent()
                .and_then(|p| p.file_name())
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            tile.warnings.extend(
                overlay
                    .warnings
                    .iter()
                    .chain(&report.warnings)
                    .map(|w| format!("Chrono {name}: {w}")),
            );
        }
    }

    /// The file a map-level data file is read from: the replacement of the last active Chrono
    /// entry that has one, else the map's own (`parklist_p*.txt`, `registrations.txt`,
    /// `unsched_trafficdens.txt`, `unsched_vehgroups.txt`, …). Names compare case-insensitively.
    #[must_use]
    pub fn data_file(&self, name: &str) -> Option<PathBuf> {
        self.chrono
            .set
            .effective_file(&self.chrono.active, name)
            .or_else(|| self.install.find_in(&self.config.map_dir, name))
    }

    /// The contents of a map-level data file in force: `ailists.cfg` with the group updates of the
    /// active entries merged in, any other file from [`Self::data_file`].
    #[must_use]
    pub fn data_bytes(&self, name: &str) -> Option<Vec<u8>> {
        if name.eq_ignore_ascii_case("ailists.cfg") {
            return self
                .chrono
                .set
                .ailists(&self.chrono.active)
                .map(String::into_bytes)
                .or_else(|| {
                    self.install
                        .find_in(&self.config.map_dir, name)
                        .and_then(|p| std::fs::read(p).ok())
                });
        }
        self.data_file(name).and_then(|p| std::fs::read(p).ok())
    }

    /// The timetable files in force (map `TTData/` overlaid by the active entries), by
    /// lower-case name. The game does not use timetables yet.
    #[must_use]
    pub fn ttdata_files(&self) -> BTreeMap<String, PathBuf> {
        self.chrono.set.ttdata(&self.chrono.active)
    }

    /// The Chrono state for [`ChronoState`].
    #[must_use]
    pub fn chrono_state(&self) -> ChronoState {
        let c = &self.chrono;
        let summary = c.set.summary(&c.active);
        let active: Vec<_> = c
            .active
            .indices()
            .iter()
            .filter_map(|&i| c.set.entries().get(i))
            .collect();
        ChronoState {
            date: c.date,
            active: active.iter().map(|e| e.rel.clone()).collect(),
            names: active.iter().map(|e| e.display_name("ENG")).collect(),
            deactivate_lines: summary.deactivate_lines,
            ticketpacks: summary.ticketpacks,
            moneysystem: summary.moneysystem,
        }
    }

    /// The live switch to `date`: plans and starts the rebuild (see the module docs). A date with
    /// the same active entries only moves the map date.
    pub(super) fn switch_chrono(
        &mut self,
        date: ChronoDate,
        player: Option<(i32, i32)>,
    ) -> SwitchOutcome {
        let old = self.chrono.active.clone();
        let new = self.chrono.set.active(date);
        let switch = self.chrono.set.switch(&old, &new);
        self.set_chrono_date(Some(date));
        if switch.is_noop() {
            tracing::info!(date = %date, "map date changed; same Chrono entries, nothing to rebuild");
            return SwitchOutcome::default();
        }
        let mut tiles = switch.rebuild_tiles();
        if switch.touches_prefix("parklist_p") {
            // Parked cars come from the list: every resident tile may change.
            self.parklists.clear();
            tiles.extend(self.resident.keys().copied());
        }
        for key in &tiles {
            self.tile_splines.remove(key);
            self.tile_objects.remove(key);
            self.tile_surfaces.remove(key);
        }
        self.spline_index = None;
        let order = rebuild_order(&tiles, |k| self.resident.contains_key(&k), player);
        let unloads: Vec<TileUnload> = order
            .iter()
            .filter_map(|k| self.unload_tile(*k, Some(*k) == player))
            .collect();
        for key in order.iter().rev() {
            self.queue.retain(|q| q != key);
            self.queue.push_front(*key);
        }
        self.chrono.rebuilding.clone_from(&order);
        tracing::info!(
            date = %date,
            toggled = switch.toggled.len(),
            changed_tiles = switch.tiles.len(),
            carve_ring = switch.carve_ring.len(),
            rebuild = order.len(),
            "Chrono switch"
        );
        SwitchOutcome {
            rebuild: order,
            unloads,
            traffic: switch.touches("ailists_#upd.cfg")
                || switch.touches("unsched_trafficdens.txt"),
        }
    }

    /// Tiles of the running switch still to rebuild (a tile is done once it is resident again or
    /// no longer queued, e.g. a tile that exists no more).
    pub(super) fn chrono_pending(&mut self) -> usize {
        let (resident, queue) = (&self.resident, &self.queue);
        let building = self.building.as_ref().map(|b| b.key);
        self.chrono.rebuilding.retain(|k| {
            let in_progress = building == Some(*k);
            in_progress || (!resident.contains_key(k) && queue.contains(k))
        });
        self.chrono.rebuilding.len()
    }
}

/// Handles [`MapDateChanged`]: the live Chrono switch.
pub fn apply_map_date_changes(
    mut changes: MessageReader<MapDateChanged>,
    job: Option<ResMut<WorldLoadJob>>,
    stats: Res<WorldStats>,
    mut rebuild: ResMut<ChronoRebuild>,
    mut pool: ResMut<TrafficPool>,
    mut commands: Commands,
) {
    let Some(change) = changes.read().last().copied() else {
        return;
    };
    let Some(mut job) = job else { return };
    let player = stats
        .position
        .vehicle
        .map(|(p, _)| p.tile)
        .or(stats.position.camera.map(|p| p.tile));
    let outcome = job.switch_chrono(chrono_date(change.new), player);
    for u in &outcome.unloads {
        u.apply(&mut commands);
    }
    if outcome.traffic {
        super::traffic::reset_pool(&mut pool);
    }
    let n = u32::try_from(outcome.rebuild.len()).unwrap_or(u32::MAX);
    rebuild.total_tiles = n;
    rebuild.pending_tiles = n;
    commands.insert_resource(job.chrono_state());
}

/// Counts [`ChronoRebuild::pending_tiles`] down as the switched tiles are rebuilt.
pub fn track_chrono_rebuild(job: Option<ResMut<WorldLoadJob>>, mut rebuild: ResMut<ChronoRebuild>) {
    if !rebuild.is_pending() {
        return;
    }
    let pending = job.map_or(0, |mut j| j.chrono_pending());
    let pending = u32::try_from(pending).unwrap_or(u32::MAX);
    if pending != rebuild.pending_tiles {
        rebuild.pending_tiles = pending;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_core::formats::chrono::ChronoEntry;
    use std::path::Path;

    type Ymd = (i32, u32, u32);

    fn entry(
        rel: &str,
        start: Ymd,
        end: Option<Ymd>,
        tiles: &[(i32, i32)],
        marked: &[(i32, i32)],
    ) -> ChronoEntry {
        let d = |(y, m, day): Ymd| ChronoDate::from_ymd(y, m, day);
        ChronoEntry {
            rel: rel.into(),
            start: Some(d(start)),
            end: end.map(d),
            tiles: tiles
                .iter()
                .map(|k| (*k, PathBuf::from(format!("{rel}/tile_{}_{}.map", k.0, k.1))))
                .collect(),
            terrain_relevant: marked.iter().copied().collect(),
            ..ChronoEntry::default()
        }
    }

    /// A synthetic two-entry map: entry A (from 1989) edits tiles (0,0) and (5,5) and marks (0,0)
    /// as terrain-relevant; entry B (1992–1993) edits (1,0).
    fn two_entry_map() -> ChronoSet {
        ChronoSet::from_entries(
            Path::new("/map"),
            vec![
                entry("0200_A", (1989, 11, 10), None, &[(0, 0), (5, 5)], &[(0, 0)]),
                entry("0300_B", (1992, 1, 1), Some((1993, 12, 31)), &[(1, 0)], &[]),
            ],
        )
    }

    fn plan(
        set: &ChronoSet,
        from: CalendarDate,
        to: CalendarDate,
        resident: &[(i32, i32)],
        player: (i32, i32),
    ) -> Vec<(i32, i32)> {
        let switch = set.switch(&set.active(chrono_date(from)), &set.active(chrono_date(to)));
        rebuild_order(
            &switch.rebuild_tiles(),
            |k| resident.contains(&k),
            Some(player),
        )
    }

    #[test]
    fn equal_active_sets_rebuild_nothing() {
        let set = two_entry_map();
        let resident: Vec<_> = (-1..=1)
            .flat_map(|x| (-1..=1).map(move |y| (x, y)))
            .collect();
        // 1986 and 1988: nothing active either time.
        assert!(set
            .switch(
                &set.active(chrono_date(CalendarDate::new(1986, 1, 1))),
                &set.active(chrono_date(CalendarDate::new(1988, 6, 1)))
            )
            .is_noop());
        assert!(plan(
            &set,
            CalendarDate::new(1990, 1, 1),
            CalendarDate::new(1991, 1, 1),
            &resident,
            (0, 0)
        )
        .is_empty());
    }

    /// 1986 → 1990 toggles A: its tiles plus the 3 × 3 ring of its marked tile; only resident tiles;
    /// the player's tile first.
    #[test]
    fn switch_rebuilds_changed_and_ring_tiles_that_are_resident_player_first() {
        let set = two_entry_map();
        let resident: Vec<_> = (0..=2).flat_map(|x| (0..=2).map(move |y| (x, y))).collect();
        let order = plan(
            &set,
            CalendarDate::new(1986, 1, 1),
            CalendarDate::new(1990, 1, 1),
            &resident,
            (1, 1),
        );
        assert_eq!(order.first(), Some(&(1, 1)), "{order:?}");
        // The ring around (0,0) that is resident: (0,0),(0,1),(1,0),(1,1); (5,5) is not resident.
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
        // 1990 → 1992 toggles B only: tile (1,0), no ring.
        let order = plan(
            &set,
            CalendarDate::new(1990, 1, 1),
            CalendarDate::new(1992, 6, 1),
            &resident,
            (2, 2),
        );
        assert_eq!(order, vec![(1, 0)]);
        // The player's tile leads even when other tiles have smaller keys.
        let order = plan(
            &set,
            CalendarDate::new(1986, 1, 1),
            CalendarDate::new(1992, 6, 1),
            &resident,
            (1, 0),
        );
        assert_eq!(order[0], (1, 0));
        assert_eq!(order.len(), 4);
    }
}
