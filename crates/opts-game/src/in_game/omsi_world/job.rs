//! Session resolution and the `WorldLoadJob` implementation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::{ContentCache, OmsiInstall};
use opts_core::formats::map::{MapManifest, MapObjectInstance, MapSplineInstance, MapTile};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

use super::vehicle::FormationCar;
use crate::catalogue_service::CatalogueService;
use crate::in_game::attachments::SplineIndex;
use crate::new_game_screen::types::GameCatalogue;
use crate::omsi_root::locate_omsi_root;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{OmsiSessionConfig, SessionRequest, WorldLoadJob};

impl WorldLoadJob {
    /// The materials of the road (spline) surfaces built so far; the weather darkens and
    /// whitens them when wet or snowy (`in_game::weather::surfaces`).
    pub fn spline_material_handles(&self) -> impl Iterator<Item = &Handle<StandardMaterial>> {
        self.spline_materials.values()
    }

    /// Root folder of the OMSI installation the session reads from.
    #[must_use]
    pub fn omsi_root(&self) -> &std::path::Path {
        self.install.root()
    }

    /// Fraction of tiles already processed (the tile in progress counts by how far it is).
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.total_tiles == 0 {
            1.0
        } else {
            let in_progress = self.building.as_ref().map_or(0.0, |b| 1.0 - b.fraction());
            let left = self.queue.len() as f32 + in_progress;
            (1.0 - left / self.total_tiles as f32).clamp(0.0, 1.0)
        }
    }

    /// Whether all queued tiles have been loaded (and none is in progress).
    #[must_use]
    pub fn tiles_done(&self) -> bool {
        self.queue.is_empty() && self.building.is_none()
    }

    /// Whether there is a tile to work on: one queued or one in progress.
    #[must_use]
    pub fn has_tile_work(&self) -> bool {
        !self.queue.is_empty() || self.building.is_some()
    }

    /// Tiles loaded so far and tiles asked for (the 3x3 neighbourhood), for the loading screen.
    #[must_use]
    pub fn tile_counts(&self) -> (usize, usize) {
        (
            self.total_tiles
                .saturating_sub(self.queue.len() + usize::from(self.building.is_some())),
            self.total_tiles,
        )
    }

    /// The first `limit` files that could not be read, for the loading report.
    #[must_use]
    pub fn failed_files(&self, limit: usize) -> Vec<String> {
        self.cache
            .stats
            .failures
            .iter()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Tiles around the start that the map lists but whose file is missing.
    #[must_use]
    pub fn tiles_listed_missing(&self) -> usize {
        self.missing_listed
    }

    /// Whether the session names a vehicle file (otherwise the map's AI list supplies one).
    #[must_use]
    pub fn has_vehicle_file(&self) -> bool {
        self.config.vehicle_path.is_some()
    }

    /// Next tile to load, for status messages.
    #[must_use]
    pub fn next_tile(&self) -> Option<(i32, i32)> {
        self.queue.front().copied()
    }
}

mod load;
mod resolve;
mod setup;
#[cfg(test)]
mod test_support;
#[cfg(test)]
pub(in crate::in_game::omsi_world) use test_support::job_for_tests;

pub use load::{begin_world_load, begin_world_load_at};
pub use resolve::resolve_session_config;
use setup::{choose_start, measure_grid, new_job};
// (no unused resolve imports)

mod lookup;
mod parked;

/// The parking list a `car_park` marker's first string value selects: blank, `0` or no value is
/// `0` (`parklist_p.txt`), a number `N` is `N` (`parklist_p_N.txt`). Anything else counts as `0`.
pub(super) fn parklist_selector(value: Option<&str>) -> u32 {
    value
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

/// File name of the parking list with this selector.
pub(super) fn parklist_file_name(selector: u32) -> String {
    if selector == 0 {
        "parklist_p.txt".to_owned()
    } else {
        format!("parklist_p_{selector}.txt")
    }
}

#[cfg(test)]
mod tests;
