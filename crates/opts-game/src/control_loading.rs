//! Control protocol of loads that end with problems (broken tiles ...): `loading.continue`,
//! `loading.accept_problems` and the `accept_problems` parameter of the loading commands.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::opt_str;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::control::PathPolicy;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::app::AppArgs;
use crate::in_game::load_report::{AcceptLoadProblems, AcceptLoadProblemsAlways, LoadReport};
use crate::in_game::types::SimulationLoadState;
use crate::new_game_screen::types::NewGameSetup;

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("loading.continue", loading_continue);
    registry.register("loading.accept_problems", loading_accept_problems);
}

/// `loading.accept_problems {always?}`: with `always` set, every later load that ends with
/// problems starts its drive without waiting for *Continue* (wizard loads too) until it is set to
/// `false`; answers the switches in force either way.
fn loading_accept_problems(world: &mut World, params: &Value) -> Outcome {
    match params.get("always") {
        None | Some(Value::Null) => {}
        Some(Value::Bool(on)) => world.insert_resource(AcceptLoadProblemsAlways(*on)),
        Some(_) => return Outcome::err(RpcError::invalid_params("`always` must be a boolean")),
    }
    let always = world
        .get_resource::<AcceptLoadProblemsAlways>()
        .is_some_and(|a| a.0);
    let next_load = world
        .get_resource::<AcceptLoadProblems>()
        .is_some_and(|a| a.0);
    let flag = world
        .get_resource::<crate::AppArgs>()
        .is_some_and(|a| a.0.accept_load_problems);
    Outcome::ok(json!({ "always": always, "next_load": next_load, "command_line": flag }))
}

/// The map folder of `scene.load_map`: a path, or a bare folder name of the installation's `maps`
/// (`Tram`, `Berlin-Spandau`).
pub(crate) fn map_dir_path(world: &World, params: &Value) -> Result<PathBuf, RpcError> {
    let Some(raw) = params
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
    else {
        return Err(RpcError::invalid_params("missing string parameter `path`"));
    };
    let in_maps = (raw.is_relative() && !raw.exists())
        .then(crate::omsi_root::known_omsi_root)
        .flatten()
        .map(|root| root.join("maps").join(&raw))
        .filter(|p| p.is_dir());
    world
        .resource::<PathPolicy>()
        .check_read(&in_maps.unwrap_or(raw))
}

/// Whether a load that ended with problems is accepted (by *Continue*, the protocol or the
/// command line) and its drive has not begun yet: the loading commands answer once it has.
pub(crate) fn accepted_and_starting(world: &World, report: &LoadReport) -> bool {
    let accepted = report.continue_requested
        || world
            .get_resource::<AcceptLoadProblems>()
            .is_some_and(|a| a.0)
        || world
            .get_resource::<AcceptLoadProblemsAlways>()
            .is_some_and(|a| a.0)
        || world
            .get_resource::<crate::AppArgs>()
            .is_some_and(|a| a.0.accept_load_problems);
    let started = world
        .get_resource::<SimulationLoadState>()
        .is_some_and(|s| s.hud_spawned);
    accepted && !started
}

/// `scene.load_map`: starts a session on a map with the given choices; answers when the drive
/// has begun (or when a load with problems waits for *Continue*).
pub(crate) fn load_map(world: &mut World, params: &Value) -> Outcome {
    let dir = match map_dir_path(world, params) {
        Ok(p) if p.is_dir() => p,
        Ok(p) => {
            return Outcome::err(RpcError::invalid_params(format!(
                "{} is not a map folder",
                p.display()
            )))
        }
        Err(e) => return Outcome::err(e),
    };
    // The choices of the drive: start point, weather, date, time, equipment options (like the
    // command line's `--entry`, `--weather`, `--date`, `--time`, `--setvar`), and the vehicle with
    // its repaint.
    let options = match crate::start_options::StartOptions::from_json(params) {
        Ok(o) => o,
        Err(e) => return Outcome::err(RpcError::invalid_params(e)),
    };
    let (vehicle, repaint) = match (opt_str(params, "vehicle"), opt_str(params, "repaint")) {
        (Ok(v), Ok(r)) => (v.map(str::to_string), r.map(str::to_string)),
        (Err(e), _) | (_, Err(e)) => return Outcome::err(e),
    };
    let vehicle_path = match vehicle {
        None => None,
        Some(v) => {
            let given = PathBuf::from(&v);
            let full = if given.is_absolute() {
                given
            } else {
                // Relative to the installation's `Vehicles` folder.
                match world.resource::<AppArgs>().0.omsi_path.clone() {
                    Some(root) => root.join("Vehicles").join(given),
                    None => given,
                }
            };
            let checked = world.resource::<PathPolicy>().check_read(&full);
            match checked {
                Ok(p) if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bus")) => Some(p),
                Ok(p) => {
                    return Outcome::err(RpcError::invalid_params(format!(
                        "{} is not a .bus file",
                        p.display()
                    )))
                }
                Err(e) => return Outcome::err(e),
            }
        }
    };
    let weather = world.get_resource::<opts_bevy::WeatherCatalogue>().cloned();
    if let Some(mut setup) = world.get_resource_mut::<NewGameSetup>() {
        if let Err(e) = options.apply(&mut setup, weather.as_ref()) {
            return Outcome::err(RpcError::invalid_params(e));
        }
    }
    if let Err(e) = accept_problems_param(world, params) {
        return Outcome::err(e);
    }
    let mut args = world.resource_mut::<AppArgs>();
    args.0.map_path = Some(dir);
    args.0.entry.clone_from(&options.entry);
    if let Some(p) = vehicle_path {
        args.0.vehicle_path = Some(p);
    }
    if let Some(r) = repaint {
        args.0.livery = Some(r);
    }
    Outcome::Wait(crate::control::restart_waiter(world))
}

/// `accept_problems: true` on a load: the drive starts without waiting for *Continue* if the load
/// ends with problems (one load only); `"always"` does so for this and every later load.
pub(crate) fn accept_problems_param(world: &mut World, params: &Value) -> Result<(), RpcError> {
    match params.get("accept_problems") {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Bool(on)) => {
            world.insert_resource(AcceptLoadProblems(*on));
            Ok(())
        }
        Some(Value::String(s)) if s == "always" => {
            world.insert_resource(AcceptLoadProblemsAlways(true));
            Ok(())
        }
        Some(_) => Err(RpcError::invalid_params(
            "`accept_problems` must be a boolean or \"always\"",
        )),
    }
}

/// `loading.continue {accept_all?}`: presses *Continue* on a loading screen that waits with
/// problems (broken tiles, a part of the vehicle that did not load). Without a waiting screen
/// it does nothing and says why. `accept_all: true` also lets the next load start by itself.
fn loading_continue(world: &mut World, params: &Value) -> Outcome {
    use crate::in_game::load_report::LoadOutcome;
    match params.get("accept_all") {
        None | Some(Value::Null) => {}
        Some(Value::Bool(on)) => world.insert_resource(AcceptLoadProblems(*on)),
        Some(_) => {
            return Outcome::err(RpcError::invalid_params("`accept_all` must be a boolean"));
        }
    }
    let started = world
        .get_resource::<SimulationLoadState>()
        .is_some_and(|s| s.hud_spawned);
    let Some(mut report) = world.get_resource_mut::<LoadReport>() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "no loading screen"));
    };
    let problems: Vec<Value> = report
        .problems
        .iter()
        .map(|p| json!({ "key": p.key, "args": p.args, "fatal": p.fatal, "detail": p.detail }))
        .collect();
    let outcome = format!("{:?}", report.outcome);
    let continued =
        report.outcome == LoadOutcome::ReadyWithProblems && !report.continue_requested && !started;
    if continued {
        report.continue_requested = true;
    }
    if report.outcome == LoadOutcome::Failed {
        return Outcome::err(RpcError::new(
            code::FAILED,
            format!("the load failed, there is nothing to continue: {outcome}"),
        ));
    }
    Outcome::ok(json!({ "continued": continued, "outcome": outcome, "problems": problems }))
}

#[cfg(test)]
mod tests;
