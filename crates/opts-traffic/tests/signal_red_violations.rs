//! M-0111 acceptance criterion 5 (red part), as a deterministic simulation: one signalled
//! crossing, 600 s of simulated time, cars arriving at random (seeded) intervals.
//!
//! Definitions:
//!
//! * an *arrival on red* is a car that comes within the light's approach distance
//!   ([`APPROACH_M`], the code's default of 60 m) of the stop line while the light shows red
//!   (`Aspect::Stop`, states 0 to 5);
//! * a *red violation* is a car that enters the governed lane (crosses the stop line) while the
//!   light shows red at that moment.
//!
//! Thresholds, fixed before the first run (none was adjusted after a run):
//!
//! * at least [`MIN_ARRIVALS_ON_RED`] arrivals on red, so the rate means something;
//! * violations below [`MAX_VIOLATION_RATE`] of the arrivals on red (criterion 5: below 1 %).
//!
//! Yellow passes are not asserted here: criterion 5's yellow part is about `[rowdy_factor]`, and
//! this crate's default `rowdy` is 0.

use std::collections::{HashMap, HashSet};

use glam::DVec2;
use opts_traffic::signals::SignalProgram;
use opts_traffic::{
    AgentId, Aspect, ConnectParams, IdmParams, LaneGraph, LaneId, LaneKind, LaneSegment, Rng,
    SignalKey, TrafficWorld,
};

const RUN_S: f64 = 600.0;
const DT: f64 = 0.1;
const APPROACH_M: f64 = 60.0;
const MIN_ARRIVALS_ON_RED: u64 = 20;
const MAX_VIOLATION_RATE: f64 = 0.01;

const KEY: SignalKey = (1, 0);
/// Green 30 s (state 6), yellow 4 s (state 9), red 26 s (state 0): a 60 s cycle.
const PHASES: [(u32, f64); 3] = [(6, 30.0), (9, 4.0), (0, 26.0)];

/// Approach road (no light), 300 m; the governed lane from its end, 200 m; the road beyond it.
fn crossing(seed: u64) -> (TrafficWorld, LaneId) {
    let approach = LaneSegment::straight(DVec2::new(0.0, 0.0), 0.0, 300.0, LaneKind::Road);
    let mut governed = LaneSegment::straight(DVec2::new(0.0, 300.0), 0.0, 200.0, LaneKind::Road);
    governed.signal = Some(KEY);
    let beyond = LaneSegment::straight(DVec2::new(0.0, 500.0), 0.0, 300.0, LaneKind::Road);
    let graph = LaneGraph::build(&[approach, governed, beyond], &ConnectParams::default());
    // The builder keeps the segments' order: lane 0 approaches, lane 1 is governed.
    let (approach_lane, stop_lane) = (LaneId(0), LaneId(1));
    assert!(
        graph.lane(stop_lane).is_some_and(|l| l.signal == Some(KEY)),
        "lane 1 is the governed lane"
    );
    let mut w = TrafficWorld::new(graph, seed);
    w.set_stuck_after(f64::INFINITY);
    let program = SignalProgram::new(PHASES.to_vec(), 0.0).expect("a program with a cycle");
    w.set_signals([(KEY, program)].into_iter().collect());
    w.set_clock(0.0);
    (w, approach_lane)
}

#[derive(Default, Debug)]
struct Tally {
    arrivals: u64,
    arrivals_on_red: u64,
    crossings: u64,
    violations: u64,
}

fn simulate(seed: u64) -> Tally {
    let (mut w, approach) = crossing(seed);
    let mut rng = Rng::new(seed ^ 0x9e37_79b9_7f4a_7c15);
    let mut next_spawn = 0.0;
    let mut approached: HashSet<AgentId> = HashSet::new();
    let mut lane_of: HashMap<AgentId, LaneId> = HashMap::new();
    let mut tally = Tally::default();
    let steps = (RUN_S / DT).round() as usize;
    let car = IdmParams::default();
    for _ in 0..steps {
        if w.clock() >= next_spawn {
            // The spawn fails when the start is occupied (a queue reaches back); no car is made.
            let _ = w.spawn(approach, 0.0, 12.0, car);
            next_spawn = w.clock() + 3.0 + 3.0 * rng.next_f64();
        }
        w.step(DT);
        let aspect = |w: &TrafficWorld| w.aspect(KEY).unwrap_or(Aspect::Off);
        let now_aspect = aspect(&w);
        let agents: Vec<(AgentId, LaneId, f64)> =
            w.agents().iter().map(|a| (a.id, a.lane, a.s)).collect();
        for (id, lane, s) in agents {
            let before = lane_of.insert(id, lane);
            // Approach: within the light's reach of the stop line on the approach lane.
            if lane == approach {
                let length = w.graph().lane(approach).map_or(0.0, |l| l.length());
                if length - s <= APPROACH_M && approached.insert(id) {
                    tally.arrivals += 1;
                    if now_aspect == Aspect::Stop {
                        tally.arrivals_on_red += 1;
                    }
                }
            }
            // Crossing: the car enters the governed lane from the approach lane.
            if before == Some(approach)
                && w.graph().lane(lane).is_some_and(|l| l.signal == Some(KEY))
            {
                tally.crossings += 1;
                if now_aspect == Aspect::Stop {
                    tally.violations += 1;
                }
            }
        }
        lane_of.retain(|id, _| w.agent(*id).is_some());
    }
    tally
}

#[test]
fn a_signalled_crossing_keeps_red_violations_below_the_stated_rate() {
    let total = simulate(7);
    let rate = if total.arrivals_on_red == 0 {
        0.0
    } else {
        total.violations as f64 / total.arrivals_on_red as f64
    };
    eprintln!(
        "600 s crossing: arrivals {}, arrivals on red {}, crossings {}, red violations {} ({:.2} %)",
        total.arrivals,
        total.arrivals_on_red,
        total.crossings,
        total.violations,
        100.0 * rate
    );
    assert!(
        total.arrivals_on_red >= MIN_ARRIVALS_ON_RED,
        "only {} arrivals on red; the run does not test the light",
        total.arrivals_on_red
    );
    assert!(
        rate < MAX_VIOLATION_RATE,
        "{} red violations of {} arrivals on red ({:.2} %), limit {:.0} %",
        total.violations,
        total.arrivals_on_red,
        100.0 * rate,
        100.0 * MAX_VIOLATION_RATE
    );
}
