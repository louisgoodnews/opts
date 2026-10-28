//! Control protocol of the bus configurations (M-0077): `config.list`, `config.save`,
//! `config.load`, `config.spawn`, `config.delete`, `config.update` and `config.duplicate`.
//!
//! A configuration is a vehicle, a repaint and equipment options saved under a name
//! ([`crate::bus_config`]). Saving takes the vehicle chosen in *Plan a drive* or, in a drive, the
//! player's vehicle; loading writes the choices into the plan (a drive is not changed; `config.spawn` builds
//! a configuration as a new session vehicle of the running drive). Every configuration is
//! checked against the installation when it is listed or loaded: what is gone is reported with a
//! code, never applied and never a crash.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Map, Value};

use crate::bus_config::{BusConfiguration, BusConfigurations, Checked};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::configs::ConfigLibrary;
use crate::new_game_screen::livery::LiveryState;
use crate::new_game_screen::planner::PlannerState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup, VehicleItem};

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

/// Repaints of a vehicle: the wizard's own discovery when it is for this vehicle, else a fresh
/// one (reads the vehicle's files).
struct Repaints {
    known: Option<(String, VehicleRepaints)>,
    cache: RefCell<HashMap<String, VehicleRepaints>>,
}

impl Repaints {
    fn new(world: &World) -> Self {
        let known = world.get_resource::<LiveryState>().and_then(|l| {
            l.repaints
                .clone()
                .map(|r| (l.vehicle.clone(), r))
                .filter(|(v, _)| !v.is_empty())
        });
        Self {
            known,
            cache: RefCell::default(),
        }
    }

    fn of(&self, v: &VehicleItem) -> VehicleRepaints {
        if let Some((id, r)) = &self.known {
            if *id == v.id {
                return r.clone();
            }
        }
        self.cache
            .borrow_mut()
            .entry(v.id.clone())
            .or_insert_with(|| vehicle_repaints(v))
            .clone()
    }
}

fn catalogue(world: &World) -> GameCatalogue {
    world
        .get_resource::<GameCatalogue>()
        .cloned()
        .unwrap_or_default()
}

fn describe(c: &BusConfiguration, cat: &GameCatalogue, checked: Option<&Checked>) -> Value {
    let setvars: Map<String, Value> = c
        .setvars
        .iter()
        .map(|(n, v)| (n.clone(), json!(v)))
        .collect();
    let mut out = json!({
        "id": c.id,
        "name": c.name,
        "vehicle": c.vehicle,
        "vehicle_name": cat.vehicles.iter().find(|v| v.id == c.vehicle).map(|v| v.full_name.clone()),
        "repaint": c.repaint,
        "setvars": setvars,
        "notes": c.notes,
        "favourite": c.favourite,
        "created": c.created,
        "last_used": c.last_used,
    });
    if let Some(ch) = checked {
        out["usable"] = json!(ch.usable());
        out["problems"] = Value::Array(
            ch.problems
                .iter()
                .map(|p| json!({ "code": p.code(), "message": p.to_string() }))
                .collect(),
        );
    }
    out
}

fn not_found(key: &str) -> RpcError {
    RpcError::new(code::NOT_FOUND, format!("no configuration `{key}`"))
}

fn list(world: &mut World, _: &Value) -> Outcome {
    let (_, store) = match open(world) {
        Ok(s) => s,
        Err(e) => return Outcome::err(e),
    };
    let cat = catalogue(world);
    let reps = Repaints::new(world);
    let items: Vec<Value> = store
        .configurations
        .iter()
        .map(|c| describe(c, &cat, Some(&c.check(&cat, |v| reps.of(v)))))
        .collect();
    Outcome::ok(json!({ "count": items.len(), "configurations": items }))
}

/// The player's vehicle in a drive: catalogue id, repaint and the user's equipment changes.
struct Driven {
    vehicle: String,
    repaint: String,
    setvars: Vec<(String, i32)>,
}

fn drive_vehicle(world: &mut World, cat: &GameCatalogue) -> Option<Result<Driven, RpcError>> {
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

fn save(world: &mut World, params: &Value) -> Outcome {
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

mod mutations;
use mutations::{delete, duplicate, load, spawn, update};
