//! The game's view of the world for captures and for the path policy: the app state, the map and
//! the vehicle, and the folders control commands may read below.

use bevy::prelude::*;
use opts_bevy::control::PathPolicy;
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::in_game::omsi_world::{WorldLoadJob, WorldStats};

/// Adds the game's view of the world to the metadata of every capture: application state, map,
/// vehicle and the tile the player (or, without a vehicle, the camera) is on.
pub(super) fn capture_metadata(world: &World, m: &mut serde_json::Map<String, Value>) {
    m.insert("app_state".into(), json!(format!("{:?}", app_state(world))));
    if let Some(stats) = world.get_resource::<WorldStats>() {
        if !stats.map_name.is_empty() {
            m.insert("map".into(), json!(stats.map_name));
        }
        if !stats.vehicle_name.is_empty() {
            m.insert("vehicle".into(), json!(stats.vehicle_name));
        }
        let tile = stats
            .position
            .vehicle
            .map(|(p, _)| p.tile)
            .or(stats.position.camera.map(|p| p.tile));
        if let Some((x, y)) = tile {
            m.insert("tile".into(), json!([x, y]));
        }
    }
}

/// Lets control commands read below the OMSI installation: the one known at startup (so
/// `scene.load_map` works from the main menu) and the one of the running session.
pub(super) fn allow_session_reads(
    job: Option<Res<WorldLoadJob>>,
    policy: Option<ResMut<PathPolicy>>,
    mut startup_done: Local<bool>,
) {
    let Some(mut policy) = policy else {
        return;
    };
    if !*startup_done {
        *startup_done = true;
        if let Some(root) = crate::omsi_root::known_omsi_root() {
            if !policy.read_roots.contains(&root) {
                policy.read_roots.push(root);
            }
        }
    }
    let Some(job) = job else {
        return;
    };
    let root = job.omsi_root().to_path_buf();
    if job.is_added() && !policy.read_roots.contains(&root) {
        policy.read_roots.push(root);
    }
}

pub(crate) fn app_state(world: &World) -> AppState {
    *world.resource::<State<AppState>>().get()
}
