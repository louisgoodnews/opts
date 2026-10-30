//! Parses textual control protocol commands into menu actions.

use bevy::prelude::*;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::types::GameCatalogue;
use crate::session_vehicles::SessionVehicles;

use super::super::route_page::run_timetable;
use super::super::{Action, MenuCommands, Page, VehicleMenu};

/// `map <text>` picks the first map that matches, `map <text> / <entry>` also the entry.
fn map_action(
    menu: &mut VehicleMenu,
    catalogue: Option<&GameCatalogue>,
    rest: &str,
) -> Option<Action> {
    let (m, entry) = rest
        .split_once('/')
        .map_or((rest, None), |(m, e)| (m, Some(e.trim())));
    if let Some(entry) = entry {
        menu.change_map.entry = Some(entry.to_owned());
    }
    catalogue
        .and_then(|c| {
            super::super::super::change_map::find_maps(c, m.trim())
                .first()
                .map(|x| x.id.clone())
        })
        .map(Action::PickMap)
}

/// `run <line file> <tour> <run>`: the line file may contain spaces, tour and run are the last two
/// words. Starts that timetable run.
fn queue_run(commands: &mut Commands, rest: &str) {
    let mut w: Vec<&str> = rest.split_whitespace().collect();
    let run = w.pop().and_then(|x| x.parse().ok());
    let tour = w.pop().and_then(|x| x.parse().ok());
    if let (Some(tour), Some(run), false) = (tour, run, w.is_empty()) {
        let line = w.join(" ");
        commands.queue(move |world: &mut World| {
            run_timetable(world, &line, tour, run);
        });
    }
}

/// `step <option> [-1|1]`: the option is the first word, the direction the last.
fn step_action(rest: &str) -> Option<Action> {
    let mut words = rest.split_whitespace();
    let delta = words
        .clone()
        .nth(1)
        .and_then(|d| d.parse().ok())
        .unwrap_or(1);
    words
        .next()
        .map(|name| Action::Step(name.to_string(), delta))
}

/// `repaint <text>` picks the first repaint of the shown vehicle whose name matches; the first one
/// is the default and has no name.
fn repaint_action(menu: &VehicleMenu, has: impl Fn(&str) -> bool) -> Option<Action> {
    menu.repaints
        .as_ref()
        .and_then(|(_, r)| r.names.iter().position(|n| has(n)))
        .map(|i| {
            let n = menu
                .repaints
                .as_ref()
                .map(|(_, r)| r.names[i].clone())
                .unwrap_or_default();
            Action::PickRepaint(if i == 0 { String::new() } else { n })
        })
}

/// The commands that name something by a piece of its text: the first configuration, vehicle,
/// repaint, entry, stop, jump target or session vehicle whose name contains it.
fn lookup_action(
    word: &str,
    rest: &str,
    menu: &VehicleMenu,
    (registry, catalogue, job): (&SessionVehicles, Option<&GameCatalogue>, Option<&WorldLoadJob>),
) -> Option<Action> {
    let has = |name: &str| name.to_lowercase().contains(rest);
    match word {
        // `renameconfig <text>` starts renaming the first configuration whose name matches.
        "renameconfig" => menu
            .configs
            .rows
            .iter()
            .find(|r| has(&r.name))
            .map(|r| Action::RenameConfig(r.id.clone())),
        // `spawnconfig <text>` spawns the first listed configuration whose name matches.
        "spawnconfig" => menu
            .configs
            .rows
            .iter()
            .find(|r| has(&r.name))
            .map(|r| Action::SpawnConfig(r.id.clone())),
        "jump" => job
            .and_then(|j| j.entry_records().into_iter().find(|(_, n)| has(n)))
            .map(|(id, _)| Action::JumpEntry(id)),
        "switch" => registry
            .records()
            .iter()
            .find(|r| r.is_alive() && (has(&r.name) || has(&r.repaint)))
            .map(|r| Action::Switch(r.id)),
        "vehicle" => catalogue
            .and_then(|c| {
                c.vehicles
                    .iter()
                    .position(|v| has(&v.full_name) || has(&v.id))
            })
            .map(Action::PickVehicle),
        "repaint" => repaint_action(menu, has),
        "entry" => menu
            .shown_entries
            .iter()
            .find(|(_, n)| has(n))
            .map(|(id, _)| Action::PickEntry(*id)),
        "stop" => menu
            .shown_stops
            .iter()
            .find(|(_, n)| has(n))
            .map(|(id, _)| Action::PickStop(*id)),
        _ => None,
    }
}

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
        let has = |name: &str| name.to_lowercase().contains(&rest);
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
            _ => lookup_action(
                word,
                &rest,
                menu,
                (registry, catalogue, job),
            ),
        };
        todo.extend(action);
    }
}
