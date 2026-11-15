//! `state`, `scene` and `scene.unload`: the state of the app, the loaded scene and leaving it.

use bevy::prelude::*;
use opts_bevy::control::handlers::query::base_state;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::SimulationLoadState;

use super::context::app_state;

pub(super) fn state(world: &mut World, _: &Value) -> Outcome {
    let mut m = base_state(world);
    m.insert("app_state".into(), json!(format!("{:?}", app_state(world))));
    if let Some(stats) = world.get_resource::<WorldStats>() {
        m.insert("map".into(), json!(stats.map_name));
        m.insert("vehicle".into(), json!(stats.vehicle_name));
    }
    if let Some(load) = world.get_resource::<SimulationLoadState>() {
        m.insert("loading".into(), json!({ "stage": format!("{:?}", load.stage), "progress": load.progress, "ready": load.hud_spawned }));
    }
    Outcome::ok(Value::Object(m))
}

pub(super) fn scene(world: &mut World, _: &Value) -> Outcome {
    let Some(stats) = world.get_resource::<WorldStats>().cloned() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    };
    let meshes = world.resource::<Assets<Mesh>>().len();
    let materials = world.resource::<Assets<StandardMaterial>>().len();
    let images = world.resource::<Assets<Image>>().len();
    let live_entities = world.query::<Entity>().iter(world).count();
    Outcome::ok(json!({
        "map": stats.map_name,
        "start_tile": [stats.start_tile.0, stats.start_tile.1],
        "tiles_loaded": stats.tiles_loaded,
        "tiles_missing": stats.tiles_missing,
        "entity_count": world.entities().len(),
        "entities_live": live_entities,
        "meshes": meshes,
        "materials": materials,
        "images": images,
        "asset_count": meshes + materials + images,
        "stats": {
            "objects_loaded": stats.objects_loaded,
            "objects_failed": stats.objects_failed,
            "objects_protected": stats.objects_protected,
            "trees": stats.trees_loaded,
            "splines_loaded": stats.splines_loaded,
            "splines_failed": stats.splines_failed,
            "scene_vertices": stats.scene_vertices,
            "asset_failures": stats.asset_failures,
            "vehicle": stats.vehicle_name,
        },
    }))
}

pub(super) fn unload(world: &mut World, _: &Value) -> Outcome {
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    Outcome::ok(json!({ "ok": true }))
}
