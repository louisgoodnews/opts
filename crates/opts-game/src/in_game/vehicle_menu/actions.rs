//! Button actions and set-variable values of the menu.

mod remote;

use super::route_page::{clear_route, jump_to_entry, pick_route, pick_route_text, pick_run};
use super::*;
use remote::parse_remote_commands;

#[allow(clippy::too_many_arguments)]
pub(super) fn menu_actions(
    mut clicks: MessageReader<ButtonClicked>,
    actions: Query<&Action>,
    mut menu: ResMut<VehicleMenu>,
    catalogue: Option<Res<GameCatalogue>>,
    job: Option<Res<WorldLoadJob>>,
    mut commands: Commands,
    mut registry: ResMut<SessionVehicles>,
    mut remote: ResMut<MenuCommands>,
    mut formation_commands: ResMut<super::super::formation_menu::FormationCommands>,
    mut library: Option<ResMut<crate::new_game_screen::configs::ConfigLibrary>>,
) {
    // The wizard's list when the game has one (one list for the screens and the protocol), else
    // one read from the file when first needed.
    let mut fallback: Option<crate::new_game_screen::configs::ConfigLibrary> = None;
    let mut todo: Vec<Action> = clicks
        .read()
        .filter_map(|ButtonClicked(e)| actions.get(*e).ok().cloned())
        .collect();
    parse_remote_commands(
        &mut remote,
        &mut menu,
        &mut formation_commands,
        catalogue.as_deref(),
        job.as_deref(),
        &registry,
        &mut commands,
        &mut todo,
    );
    for action in todo {
        match action {
            Action::Despawn => match registry.active_id() {
                None => menu.status = "There is no vehicle to despawn.".into(),
                Some(id) => {
                    crate::session_vehicles::defer(&mut commands, move |w: &mut World| {
                        if let Err(e) = ops::despawn(w, id, DespawnReason::User) {
                            tracing::warn!("despawn refused: {e}");
                        }
                    });
                    menu.current.clear();
                    menu.status = "Vehicle despawned.".into();
                }
            },
            Action::Switch(id) => {
                commands.queue(move |w: &mut World| {
                    if let Err(e) = ops::switch_to(w, id) {
                        tracing::warn!("switch refused: {e}");
                    }
                });
                menu.status.clear();
            }
            Action::DespawnId(id) => {
                commands.queue(move |w: &mut World| crate::session_vehicles::ask_despawn(w, id));
            }
            Action::Respawn(id) => {
                commands.queue(move |w: &mut World| {
                    if let Err(e) = ops::respawn(w, id) {
                        tracing::warn!("respawn refused: {e}");
                    }
                });
                menu.status = "Vehicle put back at its spawn point.".into();
            }
            Action::ResetOnRoad(id) => {
                commands.queue(move |w: &mut World| {
                    if let Err(e) = crate::session_vehicles::reset_on_road(w, id) {
                        tracing::warn!("reset on the road refused: {e}");
                    }
                });
                menu.status = "Vehicle put on the nearest road.".into();
            }
            Action::OpenSelect => {
                menu.page = Page::Select;
                menu.status.clear();
            }
            Action::OpenFormation => {
                menu.page = Page::Formation;
                menu.status.clear();
            }
            Action::Close => {
                menu.page = Page::Closed;
                menu.config_open = false;
            }
            Action::Back => {
                menu.page = if menu.page == Page::ChangeMapConfirm {
                    Page::ChangeMap
                } else {
                    Page::Main
                };
                menu.config_open = false;
            }
            Action::OpenRoute => {
                menu.page = Page::Route;
                menu.status.clear();
            }
            Action::PickRoute(code) => {
                commands.queue(move |w: &mut World| pick_route(w, code));
            }
            Action::PickRouteText(text) => {
                menu.page = Page::Route;
                commands.queue(move |w: &mut World| pick_route_text(w, &text));
            }
            Action::TtLine(i) => {
                menu.tt_line = Some(i);
                menu.tt_tour = None;
                menu.scroll[6] = 0.0;
            }
            Action::TtTour(i) => {
                menu.tt_tour = Some(i);
                menu.scroll[6] = 0.0;
            }
            Action::TtRun(i) => {
                let (Some(l), Some(t)) = (menu.tt_line, menu.tt_tour) else {
                    continue;
                };
                commands.queue(move |w: &mut World| pick_run(w, l, t, i));
            }
            Action::TtUp => {
                if menu.tt_tour.take().is_none() {
                    menu.tt_line = None;
                }
                menu.scroll[6] = 0.0;
            }
            Action::ClearRoute => {
                commands.queue(|w: &mut World| clear_route(w));
            }
            Action::OpenChangeMap => {
                menu.page = Page::ChangeMap;
                menu.status.clear();
                menu.change_map = ChangeMapDraft::default();
            }
            Action::PickMap(id) => {
                if menu.change_map.map.as_deref() != Some(id.as_str()) {
                    menu.change_map.entry = menu
                        .change_map
                        .entry
                        .take()
                        .filter(|_| menu.page == Page::ChangeMap);
                }
                menu.change_map.map = Some(id);
                menu.page = Page::ChangeMapConfirm;
                menu.status.clear();
            }
            Action::PickMapEntry(name) => menu.change_map.entry = Some(name),
            Action::DriveWithoutVehicle => menu.change_map.without_vehicle = true,
            Action::ConfirmChangeMap => {
                commands.queue(|w: &mut World| confirm_change_map(w));
            }
            Action::OpenConfigs => {
                menu.page = Page::Configs;
                menu.status.clear();
                if let Some(cat) = catalogue.as_deref() {
                    let lib = lib_of(&mut library, &mut fallback);
                    menu.configs = super::load_view(lib, cat);
                }
            }
            Action::SaveConfig => {
                if let Some(cat) = catalogue.as_deref() {
                    let lib = lib_of(&mut library, &mut fallback);
                    super::configs_page::save_active(&mut menu, lib, &registry, cat);
                }
            }
            Action::SpawnConfig(id) => {
                if let (Some(cat), Some(job)) = (catalogue.as_deref(), job.as_deref()) {
                    let lib = lib_of(&mut library, &mut fallback);
                    super::configs_page::spawn(&mut menu, lib, &mut registry, cat, job, &id);
                }
            }
            Action::OpenSetvars => {
                commands.queue(|w: &mut World| super::setvars_page::open(w));
            }
            Action::LiveToggle(name) => {
                commands.queue(move |w: &mut World| super::setvars_page::change(w, &name, None));
            }
            Action::LiveStep(name, delta) => {
                commands
                    .queue(move |w: &mut World| super::setvars_page::change(w, &name, Some(delta)));
            }
            Action::LiveReset => {
                commands.queue(|w: &mut World| super::setvars_page::reset(w));
            }
            Action::RenameConfig(id) => super::configs_page::start_rename(&mut menu, &id),
            Action::CancelRename => menu.configs.renaming = None,
            Action::CommitRename => {
                if let Some(cat) = catalogue.as_deref() {
                    let lib = lib_of(&mut library, &mut fallback);
                    super::configs_page::commit_rename(&mut menu, lib, cat);
                }
            }
            Action::DeleteConfig(id) => {
                if let Some(cat) = catalogue.as_deref() {
                    let lib = lib_of(&mut library, &mut fallback);
                    super::configs_page::delete(&mut menu, lib, cat, &id);
                }
            }
            Action::OpenJump => {
                menu.page = Page::Jump;
                menu.status.clear();
            }
            Action::JumpEntry(id) => {
                menu.page = Page::Closed;
                commands.queue(move |w: &mut World| jump_to_entry(w, id));
            }
            Action::PickVehicle(i) => {
                if menu.vehicle != Some(i) {
                    menu.vehicle = Some(i);
                    menu.repaint.clear();
                    menu.setvars.clear();
                    menu.repaints = None;
                    menu.scroll[1] = 0.0;
                    menu.scroll[3] = 0.0;
                }
            }
            Action::PickRepaint(name) => menu.repaint = name,
            Action::PickEntry(id) => (menu.entry, menu.stop) = (Some(id), None),
            Action::PickStop(id) => (menu.entry, menu.stop) = (None, Some(id)),
            Action::OpenConfig => menu.config_open = true,
            Action::CloseConfig => menu.config_open = false,
            Action::ResetConfig => menu.setvars.clear(),
            Action::Toggle(var) => {
                let current = menu_setvar_value(&menu, catalogue.as_deref(), &var);
                menu.setvars.insert(var, i32::from(current == 0));
            }
            Action::Step(var, delta) => {
                let Some(veh) = menu
                    .vehicle
                    .and_then(|i| catalogue.as_deref().and_then(|c| c.vehicles.get(i)))
                else {
                    continue;
                };
                let items = get_vehicle_setvars(veh, &menu.setvars, &menu.repaint);
                if let Some(item) = items.iter().find(|s| s.name == var) {
                    let next = item.step(item.value, delta);
                    menu.setvars.insert(var, next);
                }
            }
            Action::Confirm => {
                let (Some(cat), Some(job)) = (catalogue.as_deref(), job.as_deref()) else {
                    continue;
                };
                let Some(veh) = menu.vehicle.and_then(|i| cat.vehicles.get(i)) else {
                    menu.status = "Choose a vehicle.".into();
                    continue;
                };
                let entry = match (menu.stop, menu.entry) {
                    (Some(stop), _) => EntrySel::Stop(stop),
                    (None, Some(entry)) => EntrySel::Id(entry),
                    (None, None) => {
                        menu.status = "Choose an entry point or a bus stop.".into();
                        continue;
                    }
                };
                let Some(bus) = job.bus_file(&veh.bus_file) else {
                    tracing::error!(at = %opts_bevy::position_tag(), bus = %veh.bus_file, "vehicle .bus not found; nothing spawned");
                    menu.status = format!("ERROR: {} not found in the installation.", veh.bus_file);
                    continue;
                };
                let mut setvars: Vec<(String, i32)> =
                    menu.setvars.iter().map(|(k, v)| (k.clone(), *v)).collect();
                setvars.sort();
                registry.request_spawn(SpawnParams {
                    catalogue_id: veh.id.clone(),
                    name: veh.full_name.clone(),
                    bus_file: bus.display().to_string(),
                    repaint: menu.repaint.clone(),
                    setvars,
                    entry,
                    formation: None,
                });
                menu.current = veh.full_name.clone();
                menu.page = Page::Closed;
                menu.config_open = false;
            }
        }
    }
}

/// The configuration list the menu works on: the game's own, else one read from the file.
fn lib_of<'a>(
    library: &'a mut Option<ResMut<crate::new_game_screen::configs::ConfigLibrary>>,
    fallback: &'a mut Option<crate::new_game_screen::configs::ConfigLibrary>,
) -> &'a mut crate::new_game_screen::configs::ConfigLibrary {
    match library {
        Some(l) => l,
        None => fallback.get_or_insert_with(Default::default),
    }
}

/// Current value of an equipment option: the player's change, else the repaint's.
fn menu_setvar_value(menu: &VehicleMenu, catalogue: Option<&GameCatalogue>, var: &str) -> i32 {
    if let Some(v) = menu.setvars.get(var) {
        return *v;
    }
    let Some(veh) = menu
        .vehicle
        .and_then(|i| catalogue.and_then(|c| c.vehicles.get(i)))
    else {
        return 0;
    };
    get_vehicle_setvars(veh, &menu.setvars, &menu.repaint)
        .into_iter()
        .find(|s| s.name == var)
        .map_or(0, |s| s.value)
}
