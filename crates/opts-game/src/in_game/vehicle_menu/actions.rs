//! Button actions and set-variable values of the menu.

mod groups;
mod remote;

use super::*;
use groups::{change_map_action, navigate_action, route_action, vehicle_action};
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
            confirm_spawn(c);
            None
        }
        other => Some(other),
    }
}

/// Spawns the chosen vehicle at the chosen entry or stop, or says what is missing.
fn confirm_spawn(c: &mut MenuCtx) {
    let (Some(cat), Some(job)) = (c.catalogue, c.job) else {
        return;
    };
    let Some(veh) = c.menu.vehicle.and_then(|i| cat.vehicles.get(i)) else {
        c.menu.status = "Choose a vehicle.".into();
        return;
    };
    let entry = match (c.menu.stop, c.menu.entry) {
        (Some(stop), _) => EntrySel::Stop(stop),
        (None, Some(entry)) => EntrySel::Id(entry),
        (None, None) => {
            c.menu.status = "Choose an entry point or a bus stop.".into();
            return;
        }
    };
    let Some(bus) = job.bus_file(&veh.bus_file) else {
        tracing::error!(at = %opts_bevy::position_tag(), bus = %veh.bus_file, "vehicle .bus not found; nothing spawned");
        c.menu.status = format!("ERROR: {} not found in the installation.", veh.bus_file);
        return;
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
