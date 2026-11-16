//! The commands that name a thing by a piece of its text: a configuration, a vehicle, a repaint, an
//! entry, a stop, a jump target or a session vehicle.

use bevy::prelude::*;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::types::GameCatalogue;
use crate::session_vehicles::SessionVehicles;

use super::super::super::{Action, VehicleMenu};

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
pub(super) fn lookup_action(
    word: &str,
    rest: &str,
    menu: &VehicleMenu,
    (registry, catalogue, job): (
        &SessionVehicles,
        Option<&GameCatalogue>,
        Option<&WorldLoadJob>,
    ),
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
