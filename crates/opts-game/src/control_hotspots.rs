//! Control protocol of the cockpit hot-spots (M-0141): `vehicle.hotspots` lists the controls of
//! the player's vehicle that react to the mouse (`[mouseevent]`), `vehicle.click` operates one
//! the way a player does (press, drag, hold, release) so every control can be used and tested
//! without a mouse.

use std::collections::BTreeMap;

use bevy::prelude::*;
use opts_bevy::control::params::{opt_f64, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{MouseTrigger, PlayerVehicle, ScriptedClick, ScriptedClicks};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.hotspots", hotspots);
    registry.register("vehicle.click", click);
}

/// The hot-spots of the player's vehicle by trigger name: how many meshes carry it and where the
/// first one is (Bevy world metres).
fn collect(world: &mut World) -> BTreeMap<String, (usize, Vec3)> {
    let mut found: BTreeMap<String, (usize, Vec3)> = BTreeMap::new();
    let mut spots = world.query::<(&MouseTrigger, &GlobalTransform)>();
    let owners: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerVehicle>>()
        .iter(world)
        .collect();
    for (spot, tf) in spots.iter(world) {
        if !owners.contains(&spot.owner) {
            continue;
        }
        let e = found
            .entry(spot.trigger.clone())
            .or_insert((0, tf.translation()));
        e.0 += 1;
    }
    found
}

fn hotspots(world: &mut World, params: &Value) -> Outcome {
    let filter = match params.get("contains") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.to_lowercase(),
        Some(_) => {
            return Outcome::err(RpcError::invalid_params("`contains` must be a string"));
        }
    };
    let found = collect(world);
    if found.is_empty() && world.query::<&PlayerVehicle>().iter(world).next().is_none() {
        return Outcome::err(RpcError::new(code::NOT_READY, "no player vehicle"));
    }
    let list: Vec<Value> = found
        .iter()
        .filter(|(name, _)| name.to_lowercase().contains(&filter))
        .map(|(name, (meshes, at))| {
            json!({ "name": name, "meshes": meshes, "position": [at.x, at.y, at.z] })
        })
        .collect();
    Outcome::ok(json!({ "count": list.len(), "hotspots": list }))
}

/// `vehicle.click {name, drag_x?, drag_y?, frames?, hold_ms?}`.
fn click(world: &mut World, params: &Value) -> Outcome {
    let run = |world: &mut World| -> Result<Value, RpcError> {
        let name = str_param(params, "name")?.to_owned();
        let found = collect(world);
        let Some(exact) = found
            .keys()
            .find(|k| k.eq_ignore_ascii_case(&name))
            .cloned()
        else {
            return Err(RpcError::new(
                code::NOT_FOUND,
                format!(
                    "no hot-spot `{name}` on the player's vehicle (`vehicle.hotspots` lists them)"
                ),
            ));
        };
        let dx = opt_f64(params, "drag_x")?.unwrap_or(0.0) as f32;
        let dy = opt_f64(params, "drag_y")?.unwrap_or(0.0) as f32;
        let frames = opt_f64(params, "frames")?.unwrap_or(10.0).clamp(1.0, 600.0) as u32;
        let hold_ms = opt_f64(params, "hold_ms")?
            .unwrap_or(0.0)
            .clamp(0.0, 30_000.0);
        let click = ScriptedClick::new(&exact, Vec2::new(dx, dy), frames, hold_ms as f32 / 1000.0);
        world.resource_mut::<ScriptedClicks>().0.push(click);
        Ok(json!({
            "queued": true,
            "name": exact,
            "drag": [dx, dy],
            "drag_frames": if dx == 0.0 && dy == 0.0 { 0 } else { frames },
            "hold_ms": hold_ms,
        }))
    };
    run(world).into()
}
