//! `vehicle.set` and the session restart: a new vehicle takes effect at the next session start.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::str_param;
use opts_bevy::control::state::{Outcome, Waiter};
use opts_bevy::control::PathPolicy;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::app::AppArgs;
use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::{InGameSessionInfo, SimulationLoadState};

use super::context::app_state;

/// A waiter that drives the app through leaving the current session and entering a fresh one; it
/// finishes with a summary once the new session's HUD is up.
pub(crate) fn restart_waiter(world: &mut World) -> Waiter {
    let was_in_game = app_state(world) == AppState::InGame;
    world
        .resource_mut::<NextState<AppState>>()
        .set(if was_in_game {
            AppState::MainMenu
        } else {
            AppState::InGame
        });
    let mut left = !was_in_game;
    Box::new(move |world| {
        if !left {
            if app_state(world) != AppState::MainMenu {
                return None;
            }
            left = true;
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::InGame);
            return None;
        }
        if app_state(world) != AppState::InGame {
            return None;
        }
        // A load that failed answers at once instead of timing out; a load with problems answers
        // while the loading screen waits for *Continue* (the session is usable, `loading.continue`
        // starts it), so a script is never blocked on a person's click.
        if let Some(report) = world.get_resource::<crate::in_game::load_report::LoadReport>() {
            use crate::in_game::load_report::LoadOutcome;
            let details: Vec<String> = report.problems.iter().map(|p| p.detail.clone()).collect();
            match report.outcome {
                LoadOutcome::Failed => {
                    return Some(Err(RpcError::new(
                        code::FAILED,
                        format!("the load failed: {}", details.join("; ")),
                    )));
                }
                LoadOutcome::ReadyWithProblems => {
                    if crate::control_loading::accepted_and_starting(world, report) {
                        return None;
                    }
                    let stats = world.resource::<WorldStats>();
                    return Some(Ok(json!({
                        "map": stats.map_name, "vehicle": stats.vehicle_name,
                        "tiles_loaded": stats.tiles_loaded,
                        "waiting_for_continue": !world
                            .get_resource::<SimulationLoadState>()
                            .is_some_and(|s| s.hud_spawned),
                        "problems": details,
                    })));
                }
                _ => {}
            }
        }
        let ready = world
            .get_resource::<SimulationLoadState>()
            .is_some_and(|s| s.hud_spawned);
        ready.then(|| {
            let stats = world.resource::<WorldStats>();
            Ok(json!({ "map": stats.map_name, "vehicle": stats.vehicle_name, "tiles_loaded": stats.tiles_loaded }))
        })
    })
}

/// Brings the app into a fresh in-game session, leaving the current one first. Answers when the
/// HUD of the new session is up.
pub(super) fn restart_session(world: &mut World) -> Outcome {
    Outcome::Wait(restart_waiter(world))
}

pub(super) fn read_path(world: &World, params: &Value, name: &str) -> Result<PathBuf, RpcError> {
    let raw = str_param(params, name)?;
    world
        .resource::<PathPolicy>()
        .check_read(&PathBuf::from(raw))
}

pub(super) fn set_vehicle(world: &mut World, params: &Value) -> Outcome {
    let file = match read_path(world, params, "path") {
        Ok(p) if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bus")) => p,
        Ok(p) => {
            return Outcome::err(RpcError::invalid_params(format!(
                "{} is not a .bus file",
                p.display()
            )))
        }
        Err(e) => return Outcome::err(e),
    };
    world.resource_mut::<AppArgs>().0.vehicle_path = Some(file);
    if world.get_resource::<InGameSessionInfo>().is_none() {
        return Outcome::ok(json!({ "ok": true, "applied": "at the next session start" }));
    }
    restart_session(world)
}
