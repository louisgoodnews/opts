//! Button actions and set-variable values of the menu.

mod groups;
mod remote;

use super::*;
use groups::{change_map_action, navigate_action, route_action, vehicle_action};
use remote::{parse_remote_commands, RemoteCtx};
mod config;
use config::config_action;
mod pick;
use pick::pick_action;


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
