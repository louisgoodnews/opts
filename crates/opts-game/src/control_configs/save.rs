//! Saving the configuration of the vehicle being driven, or of the vehicle chosen in the plan
//! (`config.save`).

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, str_param};
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::bus_config::BusConfiguration;
use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};

use super::view::{catalogue, describe};
use super::{commit, now, open};

/// The player's vehicle in a drive: catalogue id, repaint and the user's equipment changes.
pub(super) struct Driven {
    pub(super) vehicle: String,
    repaint: String,
    setvars: Vec<(String, i32)>,
}

pub(super) fn drive_vehicle(
    world: &mut World,
    cat: &GameCatalogue,
) -> Option<Result<Driven, RpcError>> {
    // The active session vehicle is the player's vehicle (the load job still names the first one
    // after a switch).
    if let Some(rec) = world
        .get_resource::<crate::session_vehicles::SessionVehicles>()
        .and_then(|r| r.active())
        .filter(|r| r.is_alive())
    {
        return Some(if rec.catalogue_id.is_empty() {
            Err(RpcError::new(
                code::NOT_FOUND,
                "the driven vehicle is not in the vehicle catalogue, so it cannot be saved",
            ))
        } else {
            Ok(Driven {
                vehicle: rec.catalogue_id.clone(),
                repaint: rec.repaint.clone(),
                setvars: rec.setvars.clone(),
            })
        });
    }
    let in_drive = {
        let mut q =
            world.query_filtered::<&opts_bevy::VehicleScripts, With<opts_bevy::PlayerVehicle>>();
        q.iter(world).next().is_some()
    };
    if !in_drive {
        return None;
    }
    let job = world.get_resource::<WorldLoadJob>()?;
    let norm = |s: &str| s.replace('\\', "/").to_lowercase();
    let path = norm(&job.vehicle_file()?.to_string_lossy());
    let found = cat
        .vehicles
        .iter()
        .find(|v| !v.bus_file.is_empty() && path.ends_with(&norm(&v.bus_file)));
    Some(match found {
        Some(v) => Ok(Driven {
            vehicle: v.id.clone(),
            repaint: job.vehicle_livery().to_string(),
            setvars: job.vehicle_setvars().to_vec(),
        }),
        None => Err(RpcError::new(
            code::NOT_FOUND,
            "the driven vehicle is not in the vehicle catalogue, so it cannot be saved",
        )),
    })
}

pub(super) fn save(world: &mut World, params: &Value) -> Outcome {
    let mut run = || -> Result<Value, RpcError> {
        let name = str_param(params, "name")?.trim().to_string();
        if name.is_empty() {
            return Err(RpcError::invalid_params("parameter `name` is empty"));
        }
        let notes = opt_str(params, "notes")?.map(str::to_string);
        let replace = opt_bool(params, "replace")?.unwrap_or(false);
        let (path, mut store) = open(world)?;
        if store.find(&name).is_some() && !replace {
            return Err(RpcError::invalid_params(format!(
                "a configuration named `{name}` exists (replace: true overwrites it)"
            )));
        }
        let cat = catalogue(world);
        let mut config = match drive_vehicle(world, &cat) {
            Some(Ok(d)) => BusConfiguration::new(&name, &d.vehicle, &d.repaint, d.setvars, now()),
            Some(Err(e)) => return Err(e),
            None => {
                let setup = world.get_resource::<NewGameSetup>().ok_or_else(|| {
                    RpcError::new(
                        code::NOT_READY,
                        "no vehicle to save: start a drive or choose a vehicle in Plan a drive",
                    )
                })?;
                BusConfiguration::from_setup(&name, &cat, setup, now()).ok_or_else(|| {
                    RpcError::new(
                        code::NOT_READY,
                        "no vehicle to save: choose a vehicle in Plan a drive (a drive without a vehicle has none)",
                    )
                })?
            }
        };
        if let Some(n) = notes {
            config.notes = n;
        }
        let (id, replaced) = store.save(config);
        commit(world, &path, &store)?;
        let saved = store.find(&id).cloned().unwrap_or_default();
        Ok(json!({ "id": id, "replaced": replaced, "configuration": describe(&saved, &cat, None) }))
    };
    run().into()
}
