//! `OPTS_DBG_OBSTACLES` diagnostics of the traffic world.

/// `OPTS_DBG_OBSTACLES`: logs the obstacles and every agent that is near one.
pub(super) fn log_obstacles(world: &opts_traffic::TrafficWorld) {
    if std::env::var_os("OPTS_DBG_OBSTACLES").is_none() {
        return;
    }
    tracing::info!(
        obstacles = world.obstacles().len(),
        lanes_blocked = world.phantom_count(),
        agents = world.agents().len(),
        lanes = world.graph().len(),
        "traffic obstacles"
    );
    for a in world.agents() {
        let Some((pos, h)) = a.pose(world.graph()) else {
            continue;
        };
        for o in world.obstacles() {
            let d = pos.distance(o.centre);
            if d < 25.0 {
                tracing::info!(
                    id = a.id.0,
                    lane = a.lane.0,
                    s = a.s,
                    speed = a.speed,
                    accel = a.accel,
                    waiting = a.waiting_s,
                    ignore_until = a.ignore_conflicts_until,
                    dist = d,
                    heading = h,
                    agent_at = ?pos,
                    bus_at = ?o.centre,
                    bus_len = o.length,
                    "agent near obstacle"
                );
            }
        }
    }
}
