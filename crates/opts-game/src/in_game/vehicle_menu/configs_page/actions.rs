//! The configurations page's actions: save the active vehicle, spawn, rename and delete.

use super::super::*;
use super::rows::{load_view, now};
use crate::bus_config::{BusConfiguration, BusConfigurations};
use crate::bus_config_spawn::{from_record, spawn_params};
use crate::new_game_screen::configs::ConfigLibrary;

/// A name for the active vehicle's configuration that no saved one has.
pub(super) fn free_name(store: &BusConfigurations, base: &str) -> String {
    if store.find(base).is_none() {
        return base.to_owned();
    }
    (2..)
        .map(|n| format!("{base} ({n})"))
        .find(|name| store.find(name).is_none())
        .unwrap_or_else(|| base.to_owned())
}

/// Saves the active session vehicle as a configuration; the status line says what happened.
pub(in super::super) fn save_active(
    menu: &mut VehicleMenu,
    lib: &mut ConfigLibrary,
    registry: &SessionVehicles,
    cat: &GameCatalogue,
) {
    let Some(record) = registry.active().filter(|r| r.is_alive()) else {
        menu.status = "There is no vehicle to save.".into();
        return;
    };
    let name = free_name(&lib.store, &record.name);
    let Some(mut config) = from_record(&name, record, now()) else {
        menu.status = "This vehicle is not in the catalogue, so it cannot be saved.".into();
        return;
    };
    // A repaint the catalogue does not list (the vehicle's own default, as the `.bus` names it)
    // is saved as "the default" instead of as a repaint that later reads as missing.
    config.repaint = config.check(cat, vehicle_repaints).repaint;
    lib.store.save(config);
    menu.status = match lib.save() {
        Ok(()) => format!("Saved as {name}."),
        Err(e) => format!("Could not save: {e}"),
    };
    menu.configs = load_view(lib, cat);
}

/// Builds a configuration as a new session vehicle at the session's start point and closes the
/// menu; a refusal is shown in the status line.
pub(in super::super) fn spawn(
    menu: &mut VehicleMenu,
    lib: &mut ConfigLibrary,
    registry: &mut SessionVehicles,
    cat: &GameCatalogue,
    job: &WorldLoadJob,
    id: &str,
) {
    let Some(config): Option<BusConfiguration> = lib.store.find(id).cloned() else {
        menu.status = "That configuration is gone.".into();
        menu.configs = load_view(lib, cat);
        return;
    };
    let checked = config.check(cat, vehicle_repaints);
    match spawn_params(
        &config,
        &checked,
        cat,
        |f| job.bus_file(f),
        EntrySel::Default,
    ) {
        Ok(request) => {
            menu.current = request.name.clone();
            registry.request_spawn(request);
            lib.store.touch(&config.id, now());
            if let Err(e) = lib.save() {
                tracing::warn!("could not record the use of a configuration: {e}");
            }
            menu.page = Page::Closed;
        }
        Err(e) => menu.status = format!("Cannot spawn {}: {e}", config.name),
    }
}

/// Starts renaming a configuration: the field holds its current name.
pub(in super::super) fn start_rename(menu: &mut VehicleMenu, id: &str) {
    let name = menu
        .configs
        .rows
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.name.clone());
    menu.configs.renaming = name.map(|n| (id.to_owned(), n));
    menu.configs.confirm_delete = None;
}

/// Gives the configuration the typed name; a name that is empty or taken is refused in the
/// status line and the field stays open.
pub(in super::super) fn commit_rename(
    menu: &mut VehicleMenu,
    lib: &mut ConfigLibrary,
    cat: &GameCatalogue,
) {
    let Some((id, text)) = menu.configs.renaming.clone() else {
        return;
    };
    if !lib.store.rename(&id, &text) {
        menu.status = "That name is empty or already used.".into();
        return;
    }
    menu.status = match lib.save() {
        Ok(()) => format!("Renamed to {}.", text.trim()),
        Err(e) => format!("Could not save: {e}"),
    };
    menu.configs = load_view(lib, cat);
}

/// Deletes a configuration on the second press of its button.
pub(in super::super) fn delete(
    menu: &mut VehicleMenu,
    lib: &mut ConfigLibrary,
    cat: &GameCatalogue,
    id: &str,
) {
    if menu.configs.confirm_delete.as_deref() != Some(id) {
        menu.configs.confirm_delete = Some(id.to_owned());
        return;
    }
    lib.store.delete(id);
    menu.status = match lib.save() {
        Ok(()) => "Deleted.".into(),
        Err(e) => format!("Could not delete: {e}"),
    };
    menu.configs = load_view(lib, cat);
}
