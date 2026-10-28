//! Background discovery thread and menu selection reconciliation for the SQLite catalogue.

use super::*;

/// Keeps the current menu selection valid after the lists change.
pub fn reconcile_selection(lists: &GameCatalogue, setup: &mut NewGameSetup) {
    if !setup.no_vehicle && lists.find_vehicle(&setup.selected_vehicle).is_none() {
        if let Some(v) = lists
            .vehicles
            .iter()
            .find(|v| v.id.eq_ignore_ascii_case("MAN_SD77"))
            .or_else(|| lists.vehicles.first())
        {
            setup.selected_vehicle = lists.vehicle_key(v);
        }
    }
    if let Some(v) = lists.find_vehicle(&setup.selected_vehicle) {
        if !crate::new_game_screen::repaints::vehicle_repaints(v)
            .names
            .contains(&setup.selected_livery)
        {
            setup.selected_livery.clone_from(&v.default_livery);
        }
    }
    if let Some(map) = lists.map_by_name(&setup.selected_map) {
        if !map.entry_points.is_empty() && !map.entry_points.contains(&setup.selected_entry_point) {
            setup.selected_entry_point.clone_from(&map.entry_points[0]);
        }
    } else if let Some(m) = lists.maps.first() {
        setup.selected_map.clone_from(&m.friendly_name);
        setup.selected_entry_point = m.entry_points.first().cloned().unwrap_or_default();
    }
}

/// Plugin wiring the catalogue service into the app.
#[derive(Default)]
pub struct CataloguePlugin;

impl Plugin for CataloguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameCatalogue>()
            .init_resource::<NewGameSetup>()
            .add_systems(Startup, start_catalogue_service)
            .add_systems(Update, poll_catalogue_discovery);
    }
}

/// Starts a discovery scan on a background thread (no-op if one is already running).
pub fn spawn_discovery(service: &CatalogueService) {
    if matches!(service.status(), DiscoveryStatus::Running(_)) {
        return;
    }
    if let Ok(mut s) = service.status.lock() {
        *s = DiscoveryStatus::Running("starting");
    }
    if let Ok(mut n) = service.notified_done.lock() {
        *n = false;
    }
    let (status, root, db_path) = (
        service.status.clone(),
        service.root.clone(),
        service.db_path.clone(),
    );
    let spawned = std::thread::Builder::new()
        .name("catalogue-discovery".into())
        .spawn(move || {
            let set_phase = |p: Phase| {
                if let Ok(mut s) = status.lock() {
                    *s = DiscoveryStatus::Running(match p {
                        Phase::Walking => "scanning files",
                        Phase::Detecting => "reading content",
                        Phase::Writing => "saving",
                    });
                }
            };
            let result = opts_catalogue::discover(&db_path, &root, &set_phase);
            if let Ok(mut s) = status.lock() {
                *s = match result {
                    Ok(summary) => DiscoveryStatus::Done(summary),
                    Err(e) => DiscoveryStatus::Failed(e.to_string()),
                };
            }
        });
    if let Err(e) = spawned {
        error!("cannot start discovery thread: {e}");
    }
}
