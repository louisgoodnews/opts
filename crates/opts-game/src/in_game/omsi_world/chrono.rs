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

mod overlays;
mod data;
mod switch;
mod live;
pub use live::{apply_map_date_changes, track_chrono_rebuild};

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
}


#[cfg(test)]
mod tests;
