//! Candidate evaluation and safety checks for lane changes, split by concern.

use std::collections::HashMap;

use super::*;
use crate::agent::Agent;
use crate::graph::{Lane, LaneId};
use crate::neighbours::Side;

mod bias;
mod candidate;
mod safety;

/// The new leader (gap, speed) and the new follower (agent, gap) on a target lane.
type LeaderFollower<'a> = (Option<(f64, f64)>, Option<(&'a Agent, f64)>);

/// A neighbour lane being considered and whether the change onto it is mandatory.
struct Target<'a> {
    n: &'a crate::neighbours::Neighbour,
    lane: &'a Lane,
    mandatory: bool,
}

/// What [`TrafficWorld::evaluate`] passes to each neighbour candidate.
#[derive(Clone, Copy)]
struct EvalCtx<'a> {
    only: Option<Side>,
    gate: f64,
    a_now: f64,
    a_o_diff: f64,
    by_lane: &'a HashMap<LaneId, Vec<usize>>,
}

impl TrafficWorld {
    /// The best lane change of agent `i` that is safe (and, when `only` is `None`, worthwhile).
    pub(crate) fn evaluate(
        &self,
        i: usize,
        only: Option<Side>,
        by_lane: &HashMap<LaneId, Vec<usize>>,
    ) -> Option<Choice> {
        let a = &self.agents[i];
        let lane = self.graph.lane(a.lane)?;
        let look = self.lookahead(i);
        let own = &a.params;
        let mandatory = a.strategic.filter(|c| c.from == a.lane);
        let gate = GATE_MIN_M.max(GATE_TIME_S * a.speed);
        let inside = |l: LaneId, s: f64| {
            self.graph
                .conflicts(l)
                .iter()
                .any(|c| c.enter - 1.0 < s && s - own.length < c.exit + 1.0)
        };
        if lane.junction || inside(a.lane, a.s) {
            return None;
        }
        if mandatory.is_none() && self.junction_within(&look, gate) {
            return None;
        }
        let cap_now = self.lane_cap(own, a.lane);
        let leader_now = self.leader(i, &look, by_lane);
        let a_now = own.acceleration(a.speed, cap_now, leader_now);
        let old_follower = by_lane.get(&a.lane).and_then(|l| {
            let pos = l.iter().position(|&j| j == i)?;
            pos.checked_sub(1).map(|p| l[p])
        });
        let a_o_diff = old_follower.map_or(0.0, |j| {
            let o = &self.agents[j];
            let cap = self.lane_cap(&o.params, a.lane);
            let gap_me = a.s - own.length - o.s;
            let before = o.params.acceleration(o.speed, cap, Some((gap_me, a.speed)));
            let after_ahead = leader_now.map(|(g, v)| (g + own.length + gap_me, v));
            let after = o.params.acceleration(o.speed, cap, after_ahead);
            after - before
        });

        let ctx = EvalCtx {
            only,
            gate,
            a_now,
            a_o_diff,
            by_lane,
        };
        let mut best: Option<Choice> = None;
        for n in self.graph.neighbours(a.lane) {
            let skipped =
                only.is_some_and(|s| s != n.side) || mandatory.is_some_and(|c| c.via != n.lane);
            if !skipped {
                // A neighbour lane missing from the graph ends the whole evaluation.
                self.graph.lane(n.lane)?;
            }
            if let Some(c) = self.eval_neighbour_candidate(i, n, &ctx) {
                if best.is_none_or(|b| c.total > b.total) {
                    best = Some(c);
                }
            }
        }
        best
    }

    /// Whether a junction starts within `gate` metres along `look` (lanes with the distance
    /// from the vehicle to their start, negative for the lane it is on): the entry of a conflict
    /// zone or the start of a lane governed by a light (its stop line). Zones already passed do
    /// not count.
    pub(super) fn junction_within(&self, look: &[(LaneId, f64)], gate: f64) -> bool {
        look.iter().any(|&(l, off)| {
            if off >= gate {
                return false;
            }
            let Some(lane) = self.graph.lane(l) else {
                return false;
            };
            (off >= 0.0 && lane.signal.is_some())
                || self
                    .graph
                    .conflicts(l)
                    .iter()
                    .any(|c| (0.0..gate).contains(&(c.enter + off)))
        })
    }
}
