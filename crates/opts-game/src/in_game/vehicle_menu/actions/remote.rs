//! Parses textual control protocol commands into menu actions. The argument readers are in
//! [`args`], the lookups by a piece of a name in [`lookup`].

use bevy::prelude::*;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::types::GameCatalogue;
use crate::session_vehicles::SessionVehicles;

use super::super::{Action, MenuCommands, Page, VehicleMenu};

mod args;
mod lookup;

use args::{map_action, queue_run, step_action};
use lookup::lookup_action;

/// What [`parse_remote_commands`] reads and changes besides the command queue.
pub(super) struct RemoteCtx<'a, 'w, 's> {
    pub(super) menu: &'a mut VehicleMenu,
    pub(super) formation_commands: &'a mut super::super::super::formation_menu::FormationCommands,
    pub(super) catalogue: Option<&'a GameCatalogue>,
    pub(super) job: Option<&'a WorldLoadJob>,
    pub(super) registry: &'a SessionVehicles,
    pub(super) commands: &'a mut Commands<'w, 's>,
}

/// Turns the queued control protocol lines into menu actions (pushed on `todo`).
pub(super) fn parse_remote_commands(
    remote: &mut MenuCommands,
    ctx: RemoteCtx,
    todo: &mut Vec<Action>,
) {
    let RemoteCtx {
        menu,
        formation_commands,
        catalogue,
        job,
        registry,
        commands,
    } = ctx;
    for line in std::mem::take(&mut remote.0) {
        let (word, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        let rest = rest.trim().to_lowercase();
        let action = match word {
            "open" if menu.page == Page::Closed => {
                menu.page = Page::Main;
                None
            }
            "close" => Some(Action::Close),
            "spawn" => Some(Action::OpenSelect),
            "formation" => {
                // `formation` opens the editor; `formation <command>` is the editor's own.
                if !rest.is_empty() {
                    formation_commands.0.push(rest.clone());
                }
                (menu.page != Page::Formation).then_some(Action::OpenFormation)
            }
            "back" => Some(Action::Back),
            "changemap" => Some(Action::OpenChangeMap),
            "configs" => Some(Action::OpenConfigs),
            "saveconfig" => Some(Action::SaveConfig),
            "setvars" => Some(Action::OpenSetvars),
            "commitrename" => Some(Action::CommitRename),
            "map" => map_action(menu, catalogue, &rest),
            "route" if rest.is_empty() => Some(Action::OpenRoute),
            "route" => Some(Action::PickRouteText(rest.clone())),
            "route-clear" => Some(Action::ClearRoute),
            "jump" if rest.is_empty() => Some(Action::OpenJump),
            "run" => {
                queue_run(commands, &rest);
                None
            }
            "config" => Some(Action::OpenConfig),
            "done" => Some(Action::CloseConfig),
            "confirm" if menu.page == Page::ChangeMapConfirm => Some(Action::ConfirmChangeMap),
            "confirm" => Some(Action::Confirm),
            "despawn" => Some(Action::Despawn),
            "respawn" => registry.active_id().map(Action::Respawn),
            "reset" => registry.active_id().map(Action::ResetOnRoad),
            "toggle" => Some(Action::Toggle(rest.clone())),
            "step" => step_action(&rest),
            _ => lookup_action(word, &rest, menu, (registry, catalogue, job)),
        };
        todo.extend(action);
    }
}
