//! `scene.load_map`: the map folder a load names, and the start of the session with the choices
//! of the drive.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::opt_str;
use opts_bevy::control::state::Outcome;
use opts_bevy::control::PathPolicy;
use opts_cli::protocol::RpcError;
use serde_json::Value;

use crate::app::AppArgs;
use crate::new_game_screen::types::NewGameSetup;

use super::accept_problems_param;

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
