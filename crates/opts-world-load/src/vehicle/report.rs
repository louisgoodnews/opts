//! What the world statistics and the scene trace record about a newly loaded vehicle.

use super::*;

/// What the world statistics say about the new vehicle.
pub(super) fn record_vehicle(
    (job, stats): (&WorldLoadJob, &mut WorldStats),
    (vehicle_name, activate): (&str, bool),
    (session_id, mesh_count, vertex_count): (opts_game_state::session_vehicles::VehicleId, usize, usize),
) {
    if activate {
        stats.vehicle_name.clone_from(&vehicle_name.to_owned());
    }
    stats.vehicle_meshes = mesh_count;
    stats.vehicle_vertices = vertex_count;
    stats.scene_vertices += vertex_count;
    stats.asset_failures = job.cache.stats.failures.len();
    tracing::info!(
        vehicle = %vehicle_name,
        session_id,
        meshes = mesh_count,
        vertices = vertex_count,
        "vehicle meshes loaded"
    );
}

/// The scene trace record of the new vehicle.
pub(super) fn trace_vehicle(
    (vehicle_name, bus_path, cfg_path): (&str, &Path, &Path),
    (root, root_tf, trace_frame): (Entity, &Transform, &super::super::trace::Frame),
    (session_id, activate, mesh_count, vertex_count): (
        opts_game_state::session_vehicles::VehicleId,
        bool,
        usize,
        usize,
    ),
) {
    super::super::trace::spawned(
        opts_bevy::scene_trace::kind::VEHICLE,
        vehicle_name,
        &opts_bevy::scene_trace::relative(bus_path),
        (root, root_tf),
        trace_frame,
        "ok",
        serde_json::json!({
            "session_id": session_id, "active": activate,
            "model_cfg": opts_bevy::scene_trace::relative(cfg_path),
            "meshes": mesh_count, "vertices": vertex_count,
        }),
    );
}
