//! Where the options are read from: the wizard's choice or the driven vehicle, with its repaints and
//! its changes.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::livery::LiveryState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::session_vehicles::SessionVehicles;

/// Where the options are read from and written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Context {
    /// The session's player vehicle.
    Drive,
    /// The vehicle chosen in *Plan a drive*.
    Wizard,
}

impl Context {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Drive => "drive",
            Self::Wizard => "wizard",
        }
    }
}

/// The repaints, the chosen repaint and the user's changes of the vehicle in question.
pub(super) struct Source {
    pub(super) context: Context,
    /// The catalogue id of the vehicle (its bus file for a drive's load job); for the logs.
    pub(super) vehicle: String,
    pub(super) repaints: VehicleRepaints,
    pub(super) repaint: String,
    pub(super) custom: HashMap<String, i32>,
}

pub(super) fn source(world: &mut World) -> Option<Source> {
    let in_drive = {
        let mut q =
            world.query_filtered::<&opts_bevy::VehicleScripts, With<opts_bevy::PlayerVehicle>>();
        q.iter(world).next().is_some()
    };
    if in_drive {
        if let Some(src) = active_vehicle_source(world) {
            return Some(src);
        }
        let job = world.get_resource::<WorldLoadJob>()?;
        return Some(Source {
            context: Context::Drive,
            vehicle: job
                .vehicle_file()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            repaints: job.vehicle_repaints()?.clone(),
            repaint: job.vehicle_livery().to_string(),
            custom: job
                .vehicle_setvars()
                .iter()
                .map(|(n, v)| (n.to_lowercase(), *v))
                .collect(),
        });
    }
    let reps = world.get_resource::<LiveryState>()?.repaints.clone()?;
    let setup = world.get_resource::<NewGameSetup>()?;
    Some(Source {
        context: Context::Wizard,
        vehicle: setup.selected_vehicle.clone(),
        repaints: reps,
        repaint: setup.selected_livery.clone(),
        custom: setup
            .custom_setvars
            .iter()
            .map(|(n, v)| (n.to_lowercase(), *v))
            .collect(),
    })
}

/// The repaints and declared options of a session vehicle: the load job's own for the vehicle
/// the drive started with, else read from the vehicle's files through the catalogue.
pub(crate) fn repaints_of_record(
    world: &World,
    record: &crate::session_vehicles::SessionVehicleRecord,
) -> Option<VehicleRepaints> {
    let job = world.get_resource::<WorldLoadJob>();
    let first = job.as_ref().is_some_and(|j| {
        j.vehicle_file()
            .is_some_and(|f| f == std::path::Path::new(&record.bus_file))
    });
    if let Some(r) = job.filter(|_| first).and_then(|j| j.vehicle_repaints()) {
        return Some(r.clone());
    }
    let item = world
        .get_resource::<GameCatalogue>()?
        .vehicles
        .iter()
        .find(|v| v.id == record.catalogue_id)?;
    Some(vehicle_repaints(item))
}

/// The active session vehicle as the source of the options: its own repaint and changes (the
/// load job still describes the vehicle the drive started with, whatever was switched to since).
/// `None` when there is no session registry or no living active vehicle.
pub(super) fn active_vehicle_source(world: &World) -> Option<Source> {
    let record = world
        .get_resource::<SessionVehicles>()?
        .active()
        .filter(|r| r.is_alive())?
        .clone();
    let repaints = repaints_of_record(world, &record)?;
    Some(Source {
        context: Context::Drive,
        vehicle: record.catalogue_id.clone(),
        repaints,
        repaint: record.repaint.clone(),
        custom: record
            .setvars
            .iter()
            .map(|(n, v)| (n.to_lowercase(), *v))
            .collect(),
    })
}
