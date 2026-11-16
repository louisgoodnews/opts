//! Rebuilds the lane graph of the loaded tiles when they change.

use super::super::*;

/// Rebuilds the lane graph after tiles were loaded or unloaded; agents on lanes that no longer
/// exist are removed (their entities follow in [`move_traffic`]).
pub fn update_traffic_graph(mut state: ResMut<TrafficState>) {
    if !state.dirty {
        return;
    }
    state.dirty = false;
    let mut tiles: Vec<_> = state.segments.keys().copied().collect();
    tiles.sort_unstable();
    let all: Vec<LaneSegment> = tiles
        .iter()
        .flat_map(|k| state.segments[k].iter().cloned())
        .collect();
    let t = std::time::Instant::now();
    let graph = LaneGraph::build(&all, &ConnectParams::default());
    state.road_length_m = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .map(|l| l.length())
        .sum();
    let (lanes, length_km) = (graph.len(), state.road_length_m / 1000.0);
    let removed = state.world.replace_graph(graph);
    let programs: HashMap<SignalKey, SignalProgram> = tiles
        .iter()
        .flat_map(|k| state.signals[k].iter().cloned())
        .collect();
    let lights = programs.len();
    state.world.set_signals(programs);
    tracing::debug!(
        lanes,
        length_km,
        lights,
        removed = removed.len(),
        ms = t.elapsed().as_millis() as u64,
        "traffic lane graph rebuilt"
    );
}
