//! Control protocol of the equipment options (M-0078): `vehicle.setvars` lists the setvars of the
//! vehicle being planned (the wizard) or driven (the session's player vehicle) with kind, range,
//! value and default, and changes several of them at once, all or none.
//!
//! In the wizard a change is the same as in the equipment dialog (the choices of the plan); in a
//! drive it reaches the vehicle's scripts at once and is remembered for a respawn. See
//! `docs/src/formats/text/vehicles/setvars.md`.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::control::params::opt_bool;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_core::setvars::{Respawn, SetvarCatalogue, SetvarEntry, SetvarKind};
use serde_json::{json, Map, Value};

use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::livery::LiveryState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::setvars_discovery::catalogue_of;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::session_vehicles::SessionVehicles;

/// Registers `vehicle.setvars`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.setvars", setvars);
}

/// Where the options are read from and written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    /// The session's player vehicle.
    Drive,
    /// The vehicle chosen in *Plan a drive*.
    Wizard,
}

impl Context {
    fn name(self) -> &'static str {
        match self {
            Self::Drive => "drive",
            Self::Wizard => "wizard",
        }
    }
}

/// The repaints, the chosen repaint and the user's changes of the vehicle in question.
struct Source {
    context: Context,
    repaints: VehicleRepaints,
    repaint: String,
    custom: HashMap<String, i32>,
}

fn source(world: &mut World) -> Option<Source> {
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
fn active_vehicle_source(world: &World) -> Option<Source> {
    let record = world
        .get_resource::<SessionVehicles>()?
        .active()
        .filter(|r| r.is_alive())?
        .clone();
    let repaints = repaints_of_record(world, &record)?;
    Some(Source {
        context: Context::Drive,
        repaints,
        repaint: record.repaint.clone(),
        custom: record
            .setvars
            .iter()
            .map(|(n, v)| (n.to_lowercase(), *v))
            .collect(),
    })
}

fn kind_name(k: SetvarKind) -> &'static str {
    match k {
        SetvarKind::Switch => "switch",
        SetvarKind::Choice => "choice",
        SetvarKind::Number => "number",
        SetvarKind::Fraction => "fraction",
    }
}

fn respawn_name(r: Respawn) -> &'static str {
    match r {
        Respawn::Required => "required",
        Respawn::Live => "live",
        Respawn::Unknown => "unknown",
    }
}

/// The repaint whose values are the defaults: the chosen one, else the vehicle's default.
fn repaint_of(src: &Source) -> &str {
    if src.repaint.is_empty() {
        &src.repaints.default_name
    } else {
        &src.repaint
    }
}

fn value_of(src: &Source, e: &SetvarEntry) -> f64 {
    src.custom
        .get(&e.name.to_lowercase())
        .map_or_else(|| e.default_for(repaint_of(src)), |v| f64::from(*v))
}

fn describe(src: &Source, e: &SetvarEntry) -> Value {
    json!({
        "name": e.name,
        "label": e.label,
        "description": e.description,
        "kind": kind_name(e.kind),
        "min": e.min,
        "max": e.max,
        "values": e.values,
        "value": value_of(src, e),
        "default": e.default_for(repaint_of(src)),
        "group": e.group,
        "declared_in": e.declared_in,
        "evidence": {
            "repaints": e.evidence.observed,
            "declared": e.evidence.declared,
            "documented": e.evidence.documented,
        },
        "range_unverified": e.range_unverified,
        "respawn": respawn_name(e.respawn),
    })
}

fn setvars(world: &mut World, params: &Value) -> Outcome {
    let Some(src) = source(world) else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no vehicle to configure: start a drive or choose a vehicle in Plan a drive",
        ));
    };
    let catalogue = catalogue_of(&src.repaints);
    match params.get("values") {
        None | Some(Value::Null) => get(&src, &catalogue),
        Some(Value::Object(values)) => match opt_bool(params, "allow_unlisted") {
            Ok(allow) => set(world, &src, &catalogue, values, allow.unwrap_or(false)),
            Err(e) => Outcome::err(e),
        },
        Some(_) => Outcome::err(RpcError::invalid_params(
            "parameter `values` must be an object of option name to value",
        )),
    }
}

fn get(src: &Source, catalogue: &SetvarCatalogue) -> Outcome {
    let options: Vec<Value> = catalogue.entries.iter().map(|e| describe(src, e)).collect();
    Outcome::ok(json!({
        "context": src.context.name(),
        "repaint": repaint_of(src),
        "family": catalogue.family,
        "count": options.len(),
        "options": options,
        "not_offered": catalogue.undeclared,
    }))
}

/// What a value must be for `e`, in words.
fn expectation(e: &SetvarEntry) -> String {
    match e.kind {
        SetvarKind::Switch => "0 or 1".into(),
        SetvarKind::Choice => format!("one of {:?}", e.values),
        SetvarKind::Number => format!("a whole number from {} to {}", e.min, e.max),
        SetvarKind::Fraction => "not settable here: only the repaint sets decimal options".into(),
    }
}

/// Validates the batch against the catalogue; every refusal is listed, nothing is applied.
fn validate(
    catalogue: &SetvarCatalogue,
    values: &Map<String, Value>,
    allow_unlisted: bool,
) -> Result<Vec<(String, i32)>, Vec<Value>> {
    let (mut ok, mut bad) = (Vec::new(), Vec::new());
    for (name, v) in values {
        let refuse =
            |why: String| json!({ "key": name, "code": code::INVALID_PARAMS, "message": why });
        let number = match v {
            Value::Number(n) => n.as_f64(),
            Value::Bool(b) => Some(f64::from(u8::from(*b))),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        let Some(number) = number.filter(|n| n.is_finite() && n.fract() == 0.0) else {
            bad.push(refuse(format!("{name}: expected a whole number")));
            continue;
        };
        match catalogue.get(name) {
            Some(e) if e.accepts(number) => ok.push((e.name.clone(), number as i32)),
            Some(e) => bad.push(refuse(format!("{}: expected {}", e.name, expectation(e)))),
            None if allow_unlisted => ok.push((name.trim().to_string(), number as i32)),
            None => bad.push(refuse(format!(
                "{name}: not an option of this vehicle (allow_unlisted: true sets it anyway; the vendors warn that unlisted variables can cause faults)"
            ))),
        }
    }
    if bad.is_empty() {
        Ok(ok)
    } else {
        Err(bad)
    }
}

/// A change of the driven vehicle: set on its scripts at once, kept on its session record (the
/// save, a saved configuration and the editor read it there) and, while the driven vehicle is
/// the one the drive started with, on the load job too.
pub fn apply_live(world: &mut World, name: &str, value: i32, default: i32) {
    let active = world.get_resource::<SessionVehicles>().and_then(|r| {
        r.active()
            .filter(|r| r.is_alive())
            .map(|r| (r.id, r.bus_file.clone()))
    });
    if let Some((id, _)) = &active {
        world
            .resource_mut::<SessionVehicles>()
            .set_setvar(*id, name, value, default);
    }
    if let Some(mut job) = world.get_resource_mut::<WorldLoadJob>() {
        let first = match &active {
            Some((_, bus)) => job
                .vehicle_file()
                .is_some_and(|f| f == std::path::Path::new(bus)),
            None => true,
        };
        if first {
            job.set_vehicle_setvar(name, value);
        }
    }
    let mut q =
        world.query_filtered::<&opts_bevy::VehicleScripts, With<opts_bevy::PlayerVehicle>>();
    if let Some(scripts) = q.iter(world).next() {
        if let Ok(mut engine) = scripts.engine.lock() {
            engine.set(name, value as f32);
        }
    }
}

fn set(
    world: &mut World,
    src: &Source,
    catalogue: &SetvarCatalogue,
    values: &Map<String, Value>,
    allow_unlisted: bool,
) -> Outcome {
    if values.is_empty() {
        return Outcome::err(RpcError::invalid_params(
            "parameter `values` has no options",
        ));
    }
    let batch = match validate(catalogue, values, allow_unlisted) {
        Ok(b) => b,
        Err(bad) => {
            let first = bad[0]["message"].as_str().unwrap_or_default().to_string();
            let text = format!(
                "{} of {} options refused, nothing changed; first: {first}",
                bad.len(),
                values.len()
            );
            return Outcome::err(
                RpcError::invalid_params(text).with_data(json!({ "errors": bad })),
            );
        }
    };
    let mut changed = Vec::new();
    for (name, value) in &batch {
        let previous = catalogue.get(name).map_or(0.0, |e| value_of(src, e));
        let default = catalogue
            .get(name)
            .map_or(0, |e| e.default_for(repaint_of(src)).round() as i32);
        match src.context {
            Context::Wizard => {
                let mut setup = world.resource_mut::<NewGameSetup>();
                if *value == default {
                    setup.custom_setvars.remove(name);
                } else {
                    setup.custom_setvars.insert(name.clone(), *value);
                }
            }
            Context::Drive => apply_live(world, name, *value, default),
        }
        changed.push(json!({ "name": name, "previous": previous, "value": value }));
    }
    Outcome::ok(json!({
        "context": src.context.name(),
        "count": batch.len(),
        "changed": changed,
        "live": src.context == Context::Drive,
    }))
}
