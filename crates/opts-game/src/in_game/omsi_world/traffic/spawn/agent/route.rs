//! Routing of a scheduled vehicle: its route, its stops and, with canvases, its scripts.

use bevy::prelude::*;

use super::*;

/// Routes a scheduled vehicle: its route and stops (the first stop moved to where the front door
/// stands), the scheduled bookkeeping and, with canvases, its scripts. Returns whether the head
/// has scripts.
pub(super) fn route_scheduled(
    (job, commands, state, sched): (
        &mut WorldLoadJob,
        &mut Commands,
        &mut TrafficState,
        &mut ScheduledTraffic,
    ),
    (id, r): (AgentId, &ScheduledRequest),
    (root, door_ahead): (Entity, f32),
    (head_canvases, head_bus_path, head_setvars, sound): (
        &mut HeadCanvases,
        &std::path::PathBuf,
        &[(String, String)],
        Option<&SoundSet>,
    ),
) -> bool {
    let mut scripted_head = false;
    state.world.set_route(id, r.route.iter().copied());
    for stop in &r.stops {
        let mut stop = *stop;
        let room = state
            .world
            .graph()
            .lane(stop.lane)
            .map_or(stop.s, |l| (l.length() - 0.5).max(0.0));
        stop.s = (stop.s + f64::from(door_ahead)).min(room);
        state.world.push_stop(id, stop);
    }
    sched.built(id, r);
    if let Some(canvases) = head_canvases.take() {
        let seed = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ r.run.run as u64;
        let scripted = crate::in_game::omsi_world::traffic::ai_script::attach(
            job,
            commands,
            root,
            head_bus_path,
            (canvases, &r.run),
            (seed, head_setvars),
        );
        if let (true, Some(set)) = (scripted, sound) {
            crate::player_sound::attach_defs(commands, root, None, &set.scripted);
        }
        scripted_head = scripted;
    }
    scripted_head
}
