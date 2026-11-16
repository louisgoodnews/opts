//! The key of a drive: the map folder, the Chrono entries active on its date, and what names the map.

use std::path::PathBuf;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_core::formats::chrono::{ChronoDate, ChronoSet};
use super::super::profile::MapKey;
use crate::app::AppArgs;
use crate::new_game_screen::planner::setup_date;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};

/// The folder of the chosen map, from the command line or the OMSI installation.
fn map_dir(
    setup: &NewGameSetup,
    catalogue: Option<&GameCatalogue>,
    args: Option<&AppArgs>,
) -> Option<PathBuf> {
    if let Some(p) = args.and_then(|a| a.0.map_path.clone()) {
        return Some(p);
    }
    let id = catalogue
        .and_then(|c| c.map_by_name(&setup.selected_map))
        .map_or_else(|| setup.selected_map.clone(), |m| m.id.clone());
    Some(crate::omsi_root::known_omsi_root()?.join("maps").join(id))
}

/// The Chrono entries active on the date of the plan, as their paths below `Chrono/`, in the
/// order of the set (the newest last).
#[must_use]
pub fn active_chrono(map_dir: &std::path::Path, setup: &NewGameSetup) -> Vec<String> {
    let set = ChronoSet::scan(map_dir);
    let d = setup_date(setup);
    let active = set.active(ChronoDate::from_ymd(d.year, d.month, d.day));
    active
        .indices()
        .iter()
        .filter_map(|i| set.entries().get(*i).map(|e| e.rel.clone()))
        .collect()
}

/// The key of the plan: the map folder and the Chrono entries active on its date.
#[must_use]
pub fn key_of(
    setup: &NewGameSetup,
    catalogue: Option<&GameCatalogue>,
    args: Option<&AppArgs>,
) -> Option<MapKey> {
    let dir = map_dir(setup, catalogue, args)?;
    let name = dir.file_name()?.to_string_lossy().into_owned();
    // Several entries can be active at once (Berlin-Spandau: fifteen cumulative ones on a 1990s
    // date); the newest, the last in the order of the set, names the state of the map.
    let active = active_chrono(&dir, setup);
    let newest: Vec<&str> = active.last().map(String::as_str).into_iter().collect();
    Some(MapKey::new(&name, &newest))
}

/// What names the map of the drive: the setup, the catalogue and the command line.
#[derive(SystemParam)]
pub struct DriveChoice<'w> {
    /// The choices of the new-game wizard.
    pub setup: Res<'w, NewGameSetup>,
    /// The maps and vehicles read.
    pub catalogue: Option<Res<'w, GameCatalogue>>,
    /// The command line.
    pub args: Option<Res<'w, AppArgs>>,
}
