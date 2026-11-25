//! Logging and scene-trace records of a spawned AI vehicle.

use super::*;

/// The debug line of a spawned AI vehicle.
pub(super) fn log_spawned(
    id: AgentId,
    (name, group_name): (&str, &str),
    speed: f64,
    (params, traits): (&IdmParams, &opts_traffic::VehicleTraits),
    (units, measured, fallbacks): (usize, bool, &dyn std::fmt::Debug),
) {
    tracing::debug!(
        agent = id.0,
        vehicle = %name,
        group = %group_name,
        speed,
        mass_kg = traits.mass_kg,
        desired_kmh = params.desired_speed * 3.6,
        accel = params.max_accel,
        comfort = params.comfort_decel,
        max_decel = params.max_decel,
        class = ?params.class,
        units,
        measured,
        fallbacks = ?fallbacks,
        "AI vehicle spawned"
    );
}

/// The scene trace record of a spawned AI vehicle, when tracing.
pub(super) fn trace_spawned(
    root: Entity,
    name: &str,
    origin: &Transform,
    trace_frame: &trace::Frame,
    (lane_id, speed, params, units): (LaneId, f64, &IdmParams, usize),
) {
    if opts_bevy::scene_trace::enabled() {
        trace::spawned(
            opts_bevy::scene_trace::kind::TRAFFIC,
            name,
            name,
            (root, origin),
            trace_frame,
            "ok",
            serde_json::json!({
                "lane": format!("{lane_id:?}"),
                "speed": speed,
                "class": format!("{:?}", params.class),
                "units": units,
            }),
        );
    }
}
