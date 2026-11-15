//! The systems of the formation editor: loading the lists, typing, the buttons, the check and the
//! spawn request.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use opts_core::formats::trains::{FormationSource, TrainFormation, VehicleLookup};
use opts_gui::interaction::events::ButtonClicked;

use super::state::{Act, Car, FormationCommands, FormationEditor, ListKind, Summary};
use crate::catalogue_service::CatalogueService;
use crate::formation::store::FormationStore;
use crate::formation::{queue_spawn, resolve, FormationLibrary, JobLookup};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::vehicle_menu::VehicleMenu;
use crate::session_vehicles::{EntrySel, SessionVehicles};

mod ops;
pub(super) use ops::refresh_validation;
use ops::{delete_saved, load_preset, load_saved, save, spawn};

/// The world side of the formation actions: the load job, the vehicle registry and the library.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct FormationWorld<'w> {
    job: Option<Res<'w, WorldLoadJob>>,
    registry: ResMut<'w, SessionVehicles>,
    library: Option<Res<'w, FormationLibrary>>,
}

/// The buttons and the remote commands.
pub(super) fn formation_actions(
    mut clicks: MessageReader<ButtonClicked>,
    acts: Query<&Act>,
    mut remote: ResMut<FormationCommands>,
    mut ed: ResMut<FormationEditor>,
    mut menu: ResMut<VehicleMenu>,
    world: FormationWorld,
) {
    let FormationWorld {
        job,
        mut registry,
        library,
    } = world;
    let store = library.as_deref().and_then(|l| l.0.as_ref());
    let mut todo: Vec<Act> = clicks
        .read()
        .filter_map(|ButtonClicked(e)| acts.get(*e).ok().cloned())
        .collect();
    for line in std::mem::take(&mut remote.0) {
        let (word, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        let rest = rest.trim();
        let n = rest.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
        let act = remote_act((word, rest, n), &mut ed, &menu);
        todo.extend(act);
    }
    for act in todo {
        ed.status.clear();
        apply(
            act,
            (&mut ed, &mut menu),
            (store, job.as_deref(), &mut registry),
        );
        ed.touch();
    }
}

mod control;
mod lists;

use control::{apply, remote_act};
use lists::find_car;
#[cfg(test)]
pub(super) use lists::refresh_saved;
pub(super) use lists::{load_lists, search_input};
