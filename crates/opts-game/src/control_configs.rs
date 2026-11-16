//! Control protocol of the bus configurations (M-0077): `config.list`, `config.save`,
//! `config.load`, `config.spawn`, `config.delete`, `config.update` and `config.duplicate`.
//!
//! A configuration is a vehicle, a repaint and equipment options saved under a name
//! ([`crate::bus_config`]). Saving takes the vehicle chosen in *Plan a drive* or, in a drive, the
//! player's vehicle; loading writes the choices into the plan (a drive is not changed; `config.spawn` builds
//! a configuration as a new session vehicle of the running drive). Every configuration is
//! checked against the installation when it is listed or loaded: what is gone is reported with a
//! code, never applied and never a crash.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::bus_config::{BusConfigurations};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::configs::ConfigLibrary;
use crate::new_game_screen::livery::LiveryState;
use crate::new_game_screen::planner::PlannerState;
use crate::new_game_screen::types::{NewGameSetup};

mod save;
mod view;
use save::save;
use view::list;
use view::{catalogue, describe, not_found, Repaints};
use save::drive_vehicle;

/// Where the configurations are stored; without this resource, beside the settings database
/// ([`BusConfigurations::path`]). Tests point it into a temporary folder.
#[derive(Resource, Debug, Clone)]
pub struct BusConfigFile(pub PathBuf);

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("config.list", list);
    registry.register("config.save", save);
    registry.register("config.load", load);
    registry.register("config.delete", delete);
    registry.register("config.update", update);
    registry.register("config.duplicate", duplicate);
    registry.register("config.spawn", spawn);
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn file(world: &World) -> Result<PathBuf, RpcError> {
    world
        .get_resource::<BusConfigFile>()
        .map(|f| f.0.clone())
        .or_else(BusConfigurations::path)
        .ok_or_else(|| RpcError::new(code::FAILED, "no folder for the configurations is known"))
}

/// The configurations: the wizard's list when the game has one (the screen and a script see the
/// same list), else the file.
fn open(world: &World) -> Result<(Option<PathBuf>, BusConfigurations), RpcError> {
    if let Some(lib) = world.get_resource::<ConfigLibrary>() {
        if let Some(problem) = &lib.problem {
            return Err(RpcError::new(code::FAILED, problem.to_string()));
        }
        return Ok((lib.path.clone(), lib.store.clone()));
    }
    let path = file(world)?;
    let store = BusConfigurations::load_from(&path)
        .map_err(|e| RpcError::new(code::FAILED, e.to_string()))?;
    Ok((Some(path), store))
}

/// Makes `store` the list and writes it; the wizard builds its step again.
fn commit(
    world: &mut World,
    path: &Option<PathBuf>,
    store: &BusConfigurations,
) -> Result<(), RpcError> {
    let failed = |e: std::io::Error| {
        RpcError::new(
            code::FAILED,
            format!("cannot write the configurations: {e}"),
        )
    };
    if let Some(mut lib) = world.get_resource_mut::<ConfigLibrary>() {
        lib.store = store.clone();
        lib.pending_delete = None;
        let saved = lib.save().map_err(failed);
        if let Some(mut planner) = world.get_resource_mut::<PlannerState>() {
            planner.built = None;
        }
        return saved;
    }
    match path {
        Some(p) => store.save_to(p).map_err(failed),
        None => Ok(()),
    }
}

mod mutations;
use mutations::{delete, duplicate, load, spawn, update};
