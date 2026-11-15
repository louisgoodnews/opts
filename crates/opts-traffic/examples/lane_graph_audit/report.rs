//! Printed connectivity, conflict, light and rule figures of the lane graph.

use opts_traffic::{Aspect, LaneGraph};

use crate::files::Programs;

/// Connectivity of the lanes and the size of the graph.
pub(crate) fn print_connectivity(graph: &LaneGraph, segments: usize, ms: u128) {
    let n = graph.len();
    let with_succ = graph
        .lane_ids()
        .filter(|&l| !graph.successors(l).is_empty())
        .count();
    let mut has_pred = vec![false; n];
    for l in graph.lane_ids() {
        for s in graph.successors(l) {
            has_pred[s.0 as usize] = true;
        }
    }
    let with_pred = has_pred.iter().filter(|&&b| b).count();
    let branching = graph
        .lane_ids()
        .filter(|&l| graph.successors(l).len() > 1)
        .count();
    let total_len: f64 = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .map(|l| l.length())
        .sum();
    println!(
        "road segments: {segments}  directed lanes: {n}  total length: {:.1} km  build: {ms} ms",
        total_len / 1000.0
    );
    println!(
        "lanes with a successor: {with_succ} ({:.1} %), with a predecessor: {with_pred} ({:.1} %), branching: {branching}",
        100.0 * with_succ as f64 / n.max(1) as f64,
        100.0 * with_pred as f64 / n.max(1) as f64
    );
}

/// Conflict zones per lane and the time to compute them.
pub(crate) fn print_conflicts(graph: &LaneGraph) {
    let n = graph.len();
    let t = std::time::Instant::now();
    let conflicts: Vec<usize> = graph.lane_ids().map(|l| graph.conflicts(l).len()).collect();
    println!(
        "conflict zones: {} lanes have some ({:.1} %), {} zones in total, max {} on one lane; computed for all lanes in {} ms",
        conflicts.iter().filter(|&&c| c > 0).count(),
        100.0 * conflicts.iter().filter(|&&c| c > 0).count() as f64 / n.max(1) as f64,
        conflicts.iter().sum::<usize>(),
        conflicts.iter().max().unwrap_or(&0),
        t.elapsed().as_millis()
    );
}

/// Lanes governed by lights and the share of each aspect over all programs' cycles.
pub(crate) fn print_lights(graph: &LaneGraph, programs: &Programs) {
    let signalled = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .filter(|l| l.signal.is_some())
        .count();
    let with_program = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .filter_map(|l| l.signal)
        .filter(|k| programs.contains_key(k))
        .count();
    let mut aspects = [0usize; 4];
    for p in programs.values() {
        for t in (0..p.cycle() as usize).step_by(1) {
            aspects[match p.aspect_at(t as f64) {
                Aspect::Stop => 0,
                Aspect::Go => 1,
                Aspect::Yellow => 2,
                Aspect::Off => 3,
            }] += 1;
        }
    }
    let total: usize = aspects.iter().sum::<usize>().max(1);
    println!(
        "lights: {} programs, {signalled} lanes governed ({with_program} with a known program); share of the cycle red {:.0} %, green {:.0} %, yellow {:.0} %, dark {:.0} %",
        programs.len(),
        100.0 * aspects[0] as f64 / total as f64,
        100.0 * aspects[1] as f64 / total as f64,
        100.0 * aspects[2] as f64 / total as f64,
        100.0 * aspects[3] as f64 / total as f64,
    );
}

/// Neighbour lanes and the rules found on the lanes.
pub(crate) fn print_neighbours_and_rules(graph: &LaneGraph) {
    let n = graph.len();
    let t = std::time::Instant::now();
    let neighbours: Vec<usize> = graph
        .lane_ids()
        .map(|l| graph.neighbours(l).len())
        .collect();
    println!(
        "neighbour lanes: {} lanes have one ({:.1} %); computed for all lanes in {} ms",
        neighbours.iter().filter(|&&c| c > 0).count(),
        100.0 * neighbours.iter().filter(|&&c| c > 0).count() as f64 / n.max(1) as f64,
        t.elapsed().as_millis()
    );
    let lanes = || graph.lane_ids().filter_map(|l| graph.lane(l));
    println!(
        "rules on lanes: speed limit {}, density {}, priority {}, trucks {}, bus {}, no_cars {}, no overtaking {}",
        lanes().filter(|l| l.speed_limit.is_some()).count(),
        lanes().filter(|l| !l.rules.density.is_empty()).count(),
        lanes().filter(|l| l.rules.priority.is_some()).count(),
        lanes().filter(|l| l.rules.trucks).count(),
        lanes().filter(|l| l.rules.bus).count(),
        lanes().filter(|l| l.rules.no_cars).count(),
        lanes().filter(|l| l.rules.no_overtaking).count(),
    );
}
