//! Control protocol of the points ahead of the player's rail vehicle: `rail.switch` reads the next
//! points (distance, the ways open, the way chosen) and, with `way`, chooses the way (the same as
//! the `rail_branch_*` actions). M-0113.

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::PlayerVehicle;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::omsi_world::{points_ahead, way_by_id, way_id, RailNetwork, RailPlayer};

/// How far ahead the points are looked for (m), as the `rail_switch_toggle` action does.
const REACH_M: f32 = 150.0;

/// Registers `rail.switch`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("rail.switch", switch);
}

fn switch(world: &mut World, params: &Value) -> Outcome {
    (|| {
        let want = match params.get("way").and_then(Value::as_str) {
            Some(w) => Some(
                way_by_id(w)
                    .ok_or_else(|| RpcError::invalid_params("`way` is straight, left or right"))?,
            ),
            None => None,
        };
        world.resource_scope(|world: &mut World, net: Mut<RailNetwork>| {
            let mut q = world.query_filtered::<&mut RailPlayer, With<PlayerVehicle>>();
            let Some(mut player) = q.iter_mut(world).next() else {
                return Err(RpcError::new(
                    code::NOT_READY,
                    "the player has no rail vehicle",
                ));
            };
            let Some(train) = player.train_mut() else {
                return Ok(json!({ "placed": false, "ahead": null }));
            };
            if let Some(b) = want {
                train.branch = b;
            }
            let ahead = points_ahead(train, &net.0, REACH_M);
            // The points object ahead: its place and position, and the position the train needs.
            let object = train
                .next_switch(&net.0, REACH_M)
                .and_then(|a| net.0.lane(a.wanted)?.gate)
                .map(|g| {
                    let (x, y) = opts_sim::pedestrian::switches::switch_place(g.switch);
                    json!({
                        "x": x,
                        "y": y,
                        "position": net.0.switch_position(g.switch),
                        "needed": g.position,
                    })
                });
            Ok(json!({
                "placed": true,
                "points_objects": net.0.switches().count(),
                "points": net.0.switches().take(50).map(|k| {
                    let (x, y) = opts_sim::pedestrian::switches::switch_place(k);
                    json!({ "x": x, "y": y, "position": net.0.switch_position(k), "lanes": net.0.switch_lanes(k).len() })
                }).collect::<Vec<_>>(),
                "object": object,
                "ahead": ahead.map(|a| json!({
                    "distance_m": a.distance_m,
                    "ways": a.ways.iter().map(|b| way_id(*b)).collect::<Vec<_>>(),
                    "chosen": way_id(a.chosen),
                })),
            }))
        })
    })()
    .into()
}
