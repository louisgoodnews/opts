//! Button actions and set-variable values of the menu.

mod remote;

use super::route_page::{clear_route, jump_to_entry, pick_route, pick_route_text, pick_run};
use super::*;
use remote::{parse_remote_commands, RemoteCtx};

/// What the action handlers of the menu read and change.
struct MenuCtx<'a> {
    menu: &'a mut VehicleMenu,
    commands: &'a mut Commands<'a, 'a>,
    registry: &'a mut SessionVehicles,
    catalogue: Option<&'a GameCatalogue>,
    job: Option<&'a WorldLoadJob>,
    library: &'a mut Option<&'a mut crate::new_game_screen::configs::ConfigLibrary>,
    fallback: &'a mut Option<crate::new_game_screen::configs::ConfigLibrary>,
}

/// Despawn, switch, respawn and reset of session vehicles. Returns the action when this group does not handle it.
fn vehicle_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::Despawn => {
            match c.registry.active_id() {
                None => c.menu.status = "There is no vehicle to despawn.".into(),
                Some(id) => {
                    crate::session_vehicles::defer(c.commands, move |w: &mut World| {
                        if let Err(e) = ops::despawn(w, id, DespawnReason::User) {
                            tracing::warn!("despawn refused: {e}");
                        }
                    });
                    c.menu.current.clear();
                    c.menu.status = "Vehicle despawned.".into();
                }
            };
            None
        }
        Action::Switch(id) => {
            c.commands.queue(move |w: &mut World| {
                if let Err(e) = ops::switch_to(w, id) {
                    tracing::warn!("switch refused: {e}");
                }
            });
            c.menu.status.clear();
            None
        }
        Action::DespawnId(id) => {
            c.commands
                .queue(move |w: &mut World| crate::session_vehicles::ask_despawn(w, id));
            None
        }
        Action::Respawn(id) => {
            c.commands.queue(move |w: &mut World| {
                if let Err(e) = ops::respawn(w, id) {
                    tracing::warn!("respawn refused: {e}");
                }
            });
            c.menu.status = "Vehicle put back at its spawn point.".into();
            None
        }
        Action::ResetOnRoad(id) => {
            c.commands.queue(move |w: &mut World| {
                if let Err(e) = crate::session_vehicles::reset_on_road(w, id) {
                    tracing::warn!("reset on the road refused: {e}");
                }
            });
            c.menu.status = "Vehicle put on the nearest road.".into();
            None
        }
        other => Some(other),
    }
}

/// Opening and closing the pages of the menu. Returns the action when this group does not handle it.
fn navigate_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::OpenSelect => {
            c.menu.page = Page::Select;
            c.menu.status.clear();
            None
        }
        Action::OpenFormation => {
            c.menu.page = Page::Formation;
            c.menu.status.clear();
            None
        }
        Action::Close => {
            c.menu.page = Page::Closed;
            c.menu.config_open = false;
            None
        }
        Action::Back => {
            c.menu.page = if c.menu.page == Page::ChangeMapConfirm {
                Page::ChangeMap
            } else {
                Page::Main
            };
            c.menu.config_open = false;
            None
        }
        Action::OpenRoute => {
            c.menu.page = Page::Route;
            c.menu.status.clear();
            None
        }
        Action::OpenChangeMap => {
            c.menu.page = Page::ChangeMap;
            c.menu.status.clear();
            c.menu.change_map = ChangeMapDraft::default();
            None
        }
        Action::OpenJump => {
            c.menu.page = Page::Jump;
            c.menu.status.clear();
            None
        }
        Action::JumpEntry(id) => {
            c.menu.page = Page::Closed;
            c.commands.queue(move |w: &mut World| jump_to_entry(w, id));
            None
        }
        Action::OpenConfig => {
            c.menu.config_open = true;
            None
        }
        Action::CloseConfig => {
            c.menu.config_open = false;
            None
        }
        Action::ResetConfig => {
            c.menu.setvars.clear();
            None
        }
        other => Some(other),
    }
}

/// The route and timetable pages. Returns the action when this group does not handle it.
fn route_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::PickRoute(code) => {
            c.commands.queue(move |w: &mut World| pick_route(w, code));
            None
        }
        Action::PickRouteText(text) => {
            c.menu.page = Page::Route;
            c.commands
                .queue(move |w: &mut World| pick_route_text(w, &text));
            None
        }
        Action::TtLine(i) => {
            c.menu.tt_line = Some(i);
            c.menu.tt_tour = None;
            c.menu.scroll[6] = 0.0;
            None
        }
        Action::TtTour(i) => {
            c.menu.tt_tour = Some(i);
            c.menu.scroll[6] = 0.0;
            None
        }
        Action::TtRun(i) => {
            let (Some(l), Some(t)) = (c.menu.tt_line, c.menu.tt_tour) else {
                return None;
            };
            c.commands.queue(move |w: &mut World| pick_run(w, l, t, i));
            None
        }
        Action::TtUp => {
            if c.menu.tt_tour.take().is_none() {
                c.menu.tt_line = None;
            }
            c.menu.scroll[6] = 0.0;
            None
        }
        Action::ClearRoute => {
            c.commands.queue(|w: &mut World| clear_route(w));
            None
        }
        other => Some(other),
    }
}

/// The change-of-map pages. Returns the action when this group does not handle it.
fn change_map_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::PickMap(id) => {
            if c.menu.change_map.map.as_deref() != Some(id.as_str()) {
                c.menu.change_map.entry = c
                    .menu
                    .change_map
                    .entry
                    .take()
                    .filter(|_| c.menu.page == Page::ChangeMap);
            }
            c.menu.change_map.map = Some(id);
            c.menu.page = Page::ChangeMapConfirm;
            c.menu.status.clear();
            None
        }
        Action::PickMapEntry(name) => {
            c.menu.change_map.entry = Some(name);
            None
        }
        Action::DriveWithoutVehicle => {
            c.menu.change_map.without_vehicle = true;
            None
        }
        Action::ConfirmChangeMap => {
            c.commands.queue(|w: &mut World| confirm_change_map(w));
            None
        }
        other => Some(other),
    }
}

/// The bus configurations and the setvar editor. Returns the action when this group does not handle it.
fn config_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::OpenConfigs => {
            c.menu.page = Page::Configs;
            c.menu.status.clear();
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                c.menu.configs = super::load_view(lib, cat);
            }
            None
        }
        Action::SaveConfig => {
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::save_active(c.menu, lib, c.registry, cat);
            }
            None
        }
        Action::SpawnConfig(id) => {
            if let (Some(cat), Some(job)) = (c.catalogue, c.job) {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::spawn(c.menu, lib, c.registry, cat, job, &id);
            }
            None
        }
        Action::OpenSetvars => {
            c.commands
                .queue(|w: &mut World| super::setvars_page::open(w));
            None
        }
        Action::LiveToggle(name) => {
            c.commands
                .queue(move |w: &mut World| super::setvars_page::change(w, &name, None));
            None
        }
        Action::LiveStep(name, delta) => {
            c.commands
                .queue(move |w: &mut World| super::setvars_page::change(w, &name, Some(delta)));
            None
        }
        Action::LiveReset => {
            c.commands
                .queue(|w: &mut World| super::setvars_page::reset(w));
            None
        }
        Action::RenameConfig(id) => {
            super::configs_page::start_rename(c.menu, &id);
            None
        }
        Action::CancelRename => {
            c.menu.configs.renaming = None;
            None
        }
        Action::CommitRename => {
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::commit_rename(c.menu, lib, cat);
            }
            None
        }
        Action::DeleteConfig(id) => {
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::delete(c.menu, lib, cat, &id);
            }
            None
        }
        other => Some(other),
    }
}

/// Choosing the vehicle, repaint, entry or stop, the options, and the spawn itself. Returns the action when this group does not handle it.
fn pick_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::PickVehicle(i) => {
            if c.menu.vehicle != Some(i) {
                c.menu.vehicle = Some(i);
                c.menu.repaint.clear();
                c.menu.setvars.clear();
                c.menu.repaints = None;
                c.menu.scroll[1] = 0.0;
                c.menu.scroll[3] = 0.0;
            }
            None
        }
        Action::PickRepaint(name) => {
            c.menu.repaint = name;
            None
        }
        Action::PickEntry(id) => {
            (c.menu.entry, c.menu.stop) = (Some(id), None);
            None
        }
        Action::PickStop(id) => {
            (c.menu.entry, c.menu.stop) = (None, Some(id));
            None
        }
        Action::Toggle(var) => {
            let current = menu_setvar_value(c.menu, c.catalogue, &var);
            c.menu.setvars.insert(var, i32::from(current == 0));
            None
        }
        Action::Step(var, delta) => {
            let veh = c
                .menu
                .vehicle
                .and_then(|i| c.catalogue.and_then(|c| c.vehicles.get(i)))?;
            let items = get_vehicle_setvars(veh, &c.menu.setvars, &c.menu.repaint);
            if let Some(item) = items.iter().find(|s| s.name == var) {
                let next = item.step(item.value, delta);
                c.menu.setvars.insert(var, next);
            }
            None
        }
        Action::Confirm => {
            let (Some(cat), Some(job)) = (c.catalogue, c.job) else {
                return None;
            };
            let Some(veh) = c.menu.vehicle.and_then(|i| cat.vehicles.get(i)) else {
                c.menu.status = "Choose a vehicle.".into();
                return None;
            };
            let entry = match (c.menu.stop, c.menu.entry) {
                (Some(stop), _) => EntrySel::Stop(stop),
                (None, Some(entry)) => EntrySel::Id(entry),
                (None, None) => {
                    c.menu.status = "Choose an entry point or a bus stop.".into();
                    return None;
                }
            };
            let Some(bus) = job.bus_file(&veh.bus_file) else {
                tracing::error!(at = %opts_bevy::position_tag(), bus = %veh.bus_file, "vehicle .bus not found; nothing spawned");
                c.menu.status = format!("ERROR: {} not found in the installation.", veh.bus_file);
                return None;
            };
            let mut setvars: Vec<(String, i32)> = c
                .menu
                .setvars
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            setvars.sort();
            c.registry.request_spawn(SpawnParams {
                catalogue_id: veh.id.clone(),
                name: veh.full_name.clone(),
                bus_file: bus.display().to_string(),
                repaint: c.menu.repaint.clone(),
                setvars,
                entry,
                formation: None,
            });
            c.menu.current = veh.full_name.clone();
            c.menu.page = Page::Closed;
            c.menu.config_open = false;
            None
        }
        other => Some(other),
    }
}

/// The resources the menu actions read and change besides the menu itself.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct MenuWorld<'w> {
    catalogue: Option<Res<'w, GameCatalogue>>,
    job: Option<Res<'w, WorldLoadJob>>,
    registry: ResMut<'w, SessionVehicles>,
    remote: ResMut<'w, MenuCommands>,
    formation_commands: ResMut<'w, super::super::formation_menu::FormationCommands>,
    library: Option<ResMut<'w, crate::new_game_screen::configs::ConfigLibrary>>,
}

pub(super) fn menu_actions(
    mut clicks: MessageReader<ButtonClicked>,
    actions: Query<&Action>,
    mut menu: ResMut<VehicleMenu>,
    mut commands: Commands,
    world: MenuWorld,
) {
    let MenuWorld {
        catalogue,
        job,
        mut registry,
        mut remote,
        mut formation_commands,
        mut library,
    } = world;
    // The wizard's list when the game has one (one list for the screens and the protocol), else
    // one read from the file when first needed.
    let mut fallback: Option<crate::new_game_screen::configs::ConfigLibrary> = None;
    let mut todo: Vec<Action> = clicks
        .read()
        .filter_map(|ButtonClicked(e)| actions.get(*e).ok().cloned())
        .collect();
    parse_remote_commands(
        &mut remote,
        RemoteCtx {
            menu: &mut menu,
            formation_commands: &mut formation_commands,
            catalogue: catalogue.as_deref(),
            job: job.as_deref(),
            registry: &registry,
            commands: &mut commands,
        },
        &mut todo,
    );
    // Fresh `Commands` and plain borrows, so `MenuCtx`'s single lifetime fits.
    let mut commands = commands.reborrow();
    let mut library = library.as_deref_mut();
    let mut ctx = MenuCtx {
        menu: &mut menu,
        commands: &mut commands,
        registry: &mut registry,
        catalogue: catalogue.as_deref(),
        job: job.as_deref(),
        library: &mut library,
        fallback: &mut fallback,
    };
    for action in todo {
        let Some(action) = vehicle_action(action, &mut ctx) else {
            continue;
        };
        let Some(action) = navigate_action(action, &mut ctx) else {
            continue;
        };
        let Some(action) = route_action(action, &mut ctx) else {
            continue;
        };
        let Some(action) = change_map_action(action, &mut ctx) else {
            continue;
        };
        let Some(action) = config_action(action, &mut ctx) else {
            continue;
        };
        let Some(action) = pick_action(action, &mut ctx) else {
            continue;
        };
        let _ = action;
    }
}

/// The configuration list the menu works on: the game's own, else one read from the file.
fn lib_of<'a>(
    library: &'a mut Option<&mut crate::new_game_screen::configs::ConfigLibrary>,
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
