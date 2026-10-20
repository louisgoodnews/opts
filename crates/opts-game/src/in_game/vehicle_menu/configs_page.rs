//! The *Configurations* page of the Alt menu (M-0077): the saved bus configurations, each with
//! *Spawn* (builds it as a new session vehicle at the session's start point) and *Delete* (asks
//! with a second press), and *Save current vehicle* for the active session vehicle.

use super::build::{button, label, scroll_list};
use super::*;
use crate::bus_config::{BusConfiguration, BusConfigurations};
use crate::bus_config_spawn::{from_record, spawn_params};
use crate::new_game_screen::configs::ConfigLibrary;

/// One saved configuration as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfigRow {
    pub id: String,
    pub name: String,
    /// Vehicle name, repaint and the number of equipment options.
    pub detail: String,
    /// Why it cannot be spawned (empty: it can).
    pub blocked: String,
    /// What the check dropped but the configuration still works without.
    pub notice: String,
}

/// The page's state: the rows, the file's problem and the row that asked to be deleted.
#[derive(Debug, Clone, Default)]
pub(super) struct ConfigsView {
    pub rows: Vec<ConfigRow>,
    pub problem: Option<String>,
    pub confirm_delete: Option<String>,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// The rows of the library, each configuration checked against the installation (the repaints
/// of a vehicle are read once for all configurations that use it).
pub(super) fn load_view(lib: &ConfigLibrary, cat: &GameCatalogue) -> ConfigsView {
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
    }
}

/// The rows of a store: favourites first, then the most recently used; each configuration is
/// checked against the installation (`repaints` gives the repaints of a catalogue vehicle).
fn rows_of(
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
            let vehicle = cat
                .vehicles
                .iter()
                .find(|v| v.id == c.vehicle)
                .map_or_else(|| c.vehicle.clone(), |v| v.full_name.clone());
            let mut detail = vehicle;
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

/// The page.
pub(super) fn build_configs(commands: &mut Commands, menu: &VehicleMenu, can_save: bool) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(44.0, 80.0)
        .with_width_range(460.0, 700.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    let view = &menu.configs;
    commands.entity(content).with_children(|p| {
        label(p, "Configurations", 22.0, ColorRole::Primary);
        label(
            p,
            "A saved bus with its repaint and options. Spawn builds it as a new vehicle at the start point.",
            13.0,
            ColorRole::OnSurfaceVariant,
        );
        if let Some(problem) = &view.problem {
            label(p, problem.clone(), 13.0, ColorRole::Error);
        }
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::OnSurfaceVariant);
        }
        if view.rows.is_empty() && view.problem.is_none() {
            label(p, "No configuration is saved yet.", 13.0, ColorRole::OnSurfaceVariant);
        }
        scroll_list(p, 2, menu.scroll[2], Val::Percent(100.0), |list| {
            for row in &view.rows {
                list.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|r| {
                    label(r, row.name.clone(), 15.0, ColorRole::Primary);
                    label(r, row.detail.clone(), 12.0, ColorRole::OnSurfaceVariant);
                    if !row.blocked.is_empty() {
                        label(r, row.blocked.clone(), 12.0, ColorRole::Error);
                    } else if !row.notice.is_empty() {
                        label(r, row.notice.clone(), 12.0, ColorRole::OnSurfaceVariant);
                    }
                    r.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|buttons| {
                        if row.blocked.is_empty() {
                            let b = button(buttons, "Spawn", true, Action::SpawnConfig(row.id.clone()));
                            buttons
                                .commands()
                                .entity(b)
                                .insert(UiId::new(format!("vehicle.config.{}.spawn", row.id)));
                        }
                        let asked = view.confirm_delete.as_deref() == Some(row.id.as_str());
                        let text = if asked { "Delete? Press again" } else { "Delete" };
                        let b = button(buttons, text, asked, Action::DeleteConfig(row.id.clone()));
                        buttons
                            .commands()
                            .entity(b)
                            .insert(UiId::new(format!("vehicle.config.{}.delete", row.id)));
                    });
                });
            }
        });
        if can_save {
            let save = button(p, "Save current vehicle", false, Action::SaveConfig);
            p.commands().entity(save).insert(UiId::new("vehicle.config.save"));
        }
        button(p, "Back", false, Action::Back);
    });
}

/// A name for the active vehicle's configuration that no saved one has.
fn free_name(store: &BusConfigurations, base: &str) -> String {
    if store.find(base).is_none() {
        return base.to_owned();
    }
    (2..)
        .map(|n| format!("{base} ({n})"))
        .find(|name| store.find(name).is_none())
        .unwrap_or_else(|| base.to_owned())
}

/// Saves the active session vehicle as a configuration; the status line says what happened.
pub(super) fn save_active(
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
pub(super) fn spawn(
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

/// Deletes a configuration on the second press of its button.
pub(super) fn delete(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_game_screen::repaints::VehicleRepaints;

    fn cat() -> GameCatalogue {
        GameCatalogue {
            maps: Vec::new(),
            vehicles: vec![VehicleItem::named("c2", "Solo", "C2")],
        }
    }

    fn repaints(_: &VehicleItem) -> VehicleRepaints {
        VehicleRepaints {
            default_name: "Default".into(),
            names: vec!["Default".into(), "Red".into()],
            ..Default::default()
        }
    }

    #[test]
    fn rows_show_favourites_first_then_recent_and_block_what_cannot_spawn() {
        let mut store = BusConfigurations::default();
        let mut a = BusConfiguration::new("Old", "c2", "", vec![], 1);
        a.last_used = 10;
        store.save(a);
        let mut b = BusConfiguration::new("Recent", "c2", "Red", vec![], 1);
        b.last_used = 99;
        store.save(b);
        let (fav, _) = store.save(BusConfiguration::new("Fav", "c2", "Gone", vec![], 1));
        store.set_favourite(&fav, true);
        store.save(BusConfiguration::new("Lost", "missing", "", vec![], 1));
        let rows = rows_of(&store, &cat(), repaints);
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Fav", "Recent", "Old", "Lost"]);
        // A repaint that is gone still spawns (the default is used) and says so.
        assert!(rows[0].blocked.is_empty());
        assert!(!rows[0].notice.is_empty());
        assert!(rows[1].detail.contains("Red"));
        assert!(!rows[3].blocked.is_empty());
    }

    #[test]
    fn the_active_vehicle_gets_a_free_name() {
        let mut store = BusConfigurations::default();
        assert_eq!(free_name(&store, "Solo C2"), "Solo C2");
        store.save(BusConfiguration::new("Solo C2", "c2", "", vec![], 1));
        assert_eq!(free_name(&store, "Solo C2"), "Solo C2 (2)");
        store.save(BusConfiguration::new("Solo C2 (2)", "c2", "", vec![], 1));
        assert_eq!(free_name(&store, "solo c2"), "solo c2 (3)");
    }
}
