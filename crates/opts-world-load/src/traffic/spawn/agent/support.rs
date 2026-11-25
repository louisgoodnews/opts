//! The log line, the trace frame and the fallback-list scripts of a spawned vehicle.

use bevy::prelude::*;
use opts_traffic::{AgentId, IdmParams, LaneId};

use super::{log_spawned, trace_spawned};
use crate::traffic::ai_script;
use crate::{trace, WorldLoadJob};

/// The log line and the trace frame of a spawned vehicle.
pub(super) fn report_spawned(
    (id, lane_id, root, name, group_name): (AgentId, LaneId, Entity, &str, &str),
    (speed, params, traits, measured, fallbacks, parts): (
        f64,
        &IdmParams,
        &opts_traffic::VehicleTraits,
        bool,
        &dyn std::fmt::Debug,
        usize,
    ),
    (origin, trace_frame): (&Transform, &trace::Frame),
) {
    log_spawned(
        id,
        (name, group_name),
        speed,
        (params, traits),
        (parts, measured, fallbacks),
    );
    trace_spawned(
        root,
        name,
        origin,
        trace_frame,
        (lane_id, speed, params, parts),
    );
}

/// An ordinary vehicle on the fallback list: its script (no timetable run) drives it near the
/// camera. Its sounds stay those of an ordinary vehicle.
pub(super) fn attach_fallback(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    (id, bus_path, setvars, traits): (
        AgentId,
        &std::path::Path,
        &[(String, String)],
        &opts_traffic::VehicleTraits,
    ),
) {
    let canvases = std::sync::Arc::new(std::sync::RwLock::new(Vec::new()));
    let seed = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    if ai_script::attach(
        job,
        commands,
        root,
        bus_path,
        (canvases, None),
        (seed, setvars),
    ) {
        commands.entity(root).insert(ai_script::ScriptDrive {
            mass_kg: traits.mass_kg.unwrap_or(12_000.0),
            radius_m: traits.wheel_radius.unwrap_or(0.5),
            engaged: false,
            last_speed: None,
        });
    }
}
