//! Control protocol of the scheduled AI traffic: `traffic.scheduled` says which timetable runs
//! are on the road, which have a vehicle and how punctual the vehicles are.

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use serde_json::{json, Value};

use crate::in_game::omsi_world::{ScheduledTraffic, TrafficState};

/// Registers `traffic.scheduled`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("traffic.scheduled", scheduled);
}

fn scheduled(world: &mut World, _params: &Value) -> Outcome {
    (|| {
        let Some(s) = world.get_resource::<ScheduledTraffic>() else {
            return Ok(json!({ "available": false }));
        };
        let agents = world
            .get_resource::<TrafficState>()
            .map_or(0, |t| t.agent_count());
        Ok(json!({
            "available": true,
            "runs_on_the_road": s.active_runs(),
            "vehicles": s.driving(),
            "traffic_agents": agents,
            "spawned": s.stats.spawned,
            "arrivals": s.stats.arrivals,
            "departures": s.stats.departures,
            "missed_stops": s.stats.missed,
            "mean_delay_s": s.mean_delay_s(),
            "positions": world
                .get_resource::<crate::hud::route_path::ScheduledVehicles>()
                .map(|v| {
                    v.poses
                        .iter()
                        .map(|(p, h)| json!({ "x": p.x, "y": p.y, "heading_deg": h.to_degrees() }))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            "last_arrivals": s.stats.log.iter().map(|(station, scheduled, actual)| json!({
                "station": station,
                "scheduled": clock_text(*scheduled),
                "actual": clock_text(*actual),
                "delay_s": actual - scheduled,
            })).collect::<Vec<_>>(),
        }))
    })()
    .into()
}

fn clock_text(s: f64) -> String {
    let t = (s.max(0.0).round() as u64) % 86_400;
    format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
}
