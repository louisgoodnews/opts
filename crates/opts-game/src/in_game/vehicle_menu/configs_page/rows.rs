//! The configurations page's rows: the saved bus configurations, checked against the installation
//! and ordered for the list.

use super::super::*;
use crate::bus_config::BusConfigurations;
use crate::new_game_screen::configs::ConfigLibrary;

/// One saved configuration as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct ConfigRow {
    pub id: String,
    pub name: String,
    /// Catalogue id of the vehicle and its class (the picture slot's key and placeholder).
    pub vehicle_id: String,
    pub class: String,
    /// Vehicle name, repaint and the number of equipment options.
    pub detail: String,
    /// Why it cannot be spawned (empty: it can).
    pub blocked: String,
    /// What the check dropped but the configuration still works without.
    pub notice: String,
}

/// The page's state: the rows, the file's problem and the row that asked to be deleted.
#[derive(Debug, Clone, Default)]
pub(in super::super) struct ConfigsView {
    pub rows: Vec<ConfigRow>,
    pub problem: Option<String>,
    pub confirm_delete: Option<String>,
    /// The row being renamed and the name typed so far.
    pub renaming: Option<(String, String)>,
}

pub(super) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// The rows of the library, each configuration checked against the installation (the repaints
/// of a vehicle are read once for all configurations that use it).
pub(in super::super) fn load_view(lib: &ConfigLibrary, cat: &GameCatalogue) -> ConfigsView {
    let cache: std::cell::RefCell<HashMap<String, VehicleRepaints>> = std::cell::RefCell::default();
    let rows = rows_of(&lib.store, cat, |v| {
        cache
            .borrow_mut()
            .entry(v.id.clone())
            .or_insert_with(|| vehicle_repaints(v))
            .clone()
    });
    ConfigsView {
        rows,
        problem: lib.problem.as_ref().map(ToString::to_string),
        confirm_delete: None,
        renaming: None,
    }
}

/// The rows of a store: favourites first, then the most recently used; each configuration is
/// checked against the installation (`repaints` gives the repaints of a catalogue vehicle).
pub(super) fn rows_of(
    store: &BusConfigurations,
    cat: &GameCatalogue,
    repaints: impl Fn(&VehicleItem) -> VehicleRepaints,
) -> Vec<ConfigRow> {
    // Favourites first, then the most recently used.
    let mut ordered: Vec<(bool, i64, ConfigRow)> = store
        .configurations
        .iter()
        .map(|c| {
            let checked = c.check(cat, &repaints);
            let item = cat.vehicles.iter().find(|v| v.id == c.vehicle);
            let class = item.map(|v| v.class.clone()).unwrap_or_default();
            let mut detail = item.map_or_else(|| c.vehicle.clone(), |v| v.full_name.clone());
            if !c.repaint.is_empty() {
                detail = format!("{detail}, {}", c.repaint);
            }
            if !c.setvars.is_empty() {
                detail = format!("{detail}, {} options", c.setvars.len());
            }
            let blocked = checked
                .problems
                .iter()
                .find(|p| p.is_fatal())
                .map(ToString::to_string)
                .unwrap_or_default();
            let notice = if blocked.is_empty() {
                checked
                    .problems
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            } else {
                String::new()
            };
            let row = ConfigRow {
                id: c.id.clone(),
                name: c.name.clone(),
                vehicle_id: c.vehicle.clone(),
                class,
                detail,
                blocked,
                notice,
            };
            (c.favourite, c.last_used, row)
        })
        .collect();
    ordered.sort_by_key(|(fav, used, _)| (std::cmp::Reverse(*fav), std::cmp::Reverse(*used)));
    ordered.into_iter().map(|(_, _, r)| r).collect()
}
