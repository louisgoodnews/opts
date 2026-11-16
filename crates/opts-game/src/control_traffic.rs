//! Control protocol of the AI road traffic (M-0049): `traffic.dump` lists the vehicles on the lanes
//! and `traffic.set_density` sets the baseline the spawner aims at (the setting `gameplay.traffic`
//! for this session). Both serve the debug overlay's checks and tools.

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_traffic::{Agent, LaneGraph};
use serde_json::{json, Value};

use crate::in_game::omsi_world::{TrafficSettings, TrafficState};

/// Vehicles `traffic.dump` lists when no `limit` is given.
const DUMP_DEFAULT: u64 = 50;
/// Most vehicles `traffic.dump` lists.
const DUMP_MAX: u64 = 500;
/// Largest baseline `traffic.set_density` accepts.
pub const BASELINE_MAX: u64 = 1000;

/// Registers `traffic.dump` and `traffic.set_density`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("traffic.dump", dump);
    registry.register("traffic.set_density", set_density);
}

/// The whole number `params[name]` when there is one: `Ok(None)` when it is absent, `Err` when it
/// is not a whole number from `min` to `max`.
fn int_param(params: &Value, name: &str, min: u64, max: u64) -> Result<Option<u64>, RpcError> {
    let Some(v) = params.get(name) else {
        return Ok(None);
    };
    match v.as_u64() {
        Some(n) if (min..=max).contains(&n) => Ok(Some(n)),
        _ => Err(RpcError::new(
            code::INVALID_PARAMS,
            format!("{name} must be a whole number from {min} to {max}"),
        )),
    }
}

/// One vehicle as the dump shows it; the pose is `null` when its lane is gone.
#[must_use]
fn agent_json(a: &Agent, graph: &LaneGraph) -> Value {
    let pose = a.pose(graph);
    json!({
        "id": a.id.0,
        "lane": a.lane.0,
        "s": a.s,
        "speed_ms": a.speed,
        "accel": a.accel,
        "waiting_s": a.waiting_s,
        "stuck_s": a.stuck_s,
        "changing_lane": a.changing_lane(),
        "lateral": a.lateral,
        "x": pose.map(|(p, _)| p.x),
        "y": pose.map(|(p, _)| p.y),
        "heading_deg": pose.map(|(_, h)| h.to_degrees()),
    })
}

fn dump(world: &mut World, params: &Value) -> Outcome {
    (|| -> Result<Value, RpcError> {
        let limit = int_param(params, "limit", 1, DUMP_MAX)?.unwrap_or(DUMP_DEFAULT);
        let baseline = world.get_resource::<TrafficSettings>().map(|s| s.baseline);
        let Some(traffic) = world.get_resource::<TrafficState>() else {
            return Ok(json!({ "available": false }));
        };
        let graph = traffic.lane_graph();
        let agents = traffic.agents();
        let shown: Vec<Value> = agents
            .iter()
            .take(limit as usize)
            .map(|a| agent_json(a, graph))
            .collect();
        Ok(json!({
            "available": true,
            "baseline": baseline,
            "lanes": graph.len(),
            "agents": agents.len(),
            "shown": shown,
        }))
    })()
    .into()
}

fn set_density(world: &mut World, params: &Value) -> Outcome {
    (|| -> Result<Value, RpcError> {
        let Some(baseline) = int_param(params, "baseline", 0, BASELINE_MAX)? else {
            return Err(RpcError::new(code::INVALID_PARAMS, "baseline is required"));
        };
        let Some(mut settings) = world.get_resource_mut::<TrafficSettings>() else {
            return Err(RpcError::new(
                code::NOT_READY,
                "there is no traffic world (start a drive first)",
            ));
        };
        let previous = settings.baseline;
        settings.baseline = baseline as usize;
        Ok(json!({ "baseline": baseline, "previous": previous }))
    })()
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_whole_number_is_none() {
        assert_eq!(int_param(&json!({}), "limit", 1, 5).ok(), Some(None));
    }

    #[test]
    fn a_whole_number_in_range_is_taken() {
        assert_eq!(
            int_param(&json!({"limit": 5}), "limit", 1, 5).ok(),
            Some(Some(5))
        );
        assert_eq!(
            int_param(&json!({"baseline": 0}), "baseline", 0, 9).ok(),
            Some(Some(0))
        );
    }

    #[test]
    fn a_number_outside_the_range_or_not_whole_is_refused() {
        for bad in [
            json!({"limit": 0}),
            json!({"limit": 6}),
            json!({"limit": 1.5}),
            json!({"limit": "3"}),
            json!({"limit": -1}),
        ] {
            let err = int_param(&bad, "limit", 1, 5).expect_err("refused");
            assert_eq!(err.code, code::INVALID_PARAMS);
        }
    }
}
