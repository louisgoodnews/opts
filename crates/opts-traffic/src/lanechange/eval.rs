//! Candidate evaluation and safety checks for lane changes.

use std::collections::HashMap;

use super::*;
use crate::agent::Agent;
use crate::graph::{Lane, LaneId};
use crate::neighbours::Side;

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
    fn check_target_safety(
        &self,
        a: &Agent,
        n_lane: LaneId,
        leader_t: Option<(f64, f64)>,
        follower_t: Option<(&Agent, f64)>,
    ) -> Option<(f64, f64, f64)> {
        let (a_f_before, a_f_after) = match follower_t {
            Some((f, gap)) => {
                if gap < f.params.min_gap + 0.5 {
                    return None;
                }
                let after = f.params.acceleration(
                    f.speed,
                    self.lane_cap(&f.params, n_lane),
                    Some((gap, a.speed)),
                );
                if after < -B_SAFE_FOLLOWER {
                    return None;
                }
                let lead_gap = leader_t.map(|(g, v)| (g + a.params.length + gap, v));
                (
                    f.params
                        .acceleration(f.speed, self.lane_cap(&f.params, n_lane), lead_gap),
                    after,
                )
            }
            None => (0.0, 0.0),
        };
        if leader_t.is_some_and(|(g, _)| g < a.params.min_gap + 1.0) {
            return None;
        }
        let cap_t = self.lane_cap(&a.params, n_lane);
        let a_after = a.params.acceleration(a.speed, cap_t, leader_t);
        if a_after < -B_SAFE_SELF {
            return None;
        }
        Some((a_after, a_f_before, a_f_after))
    }

    fn find_target_leader_follower<'a>(
        &'a self,
        n_lane: LaneId,
        ml_length: f64,
        s_t: f64,
        own_length: f64,
        list: &'a [usize],
        by_lane: &'a HashMap<LaneId, Vec<usize>>,
    ) -> LeaderFollower<'a> {
        let leader_t = list
            .iter()
            .map(|&j| &self.agents[j])
            .filter(|o| o.s - o.params.length >= s_t)
            .min_by(|x, y| x.s.total_cmp(&y.s))
            .map(|o| (o.s - o.params.length - s_t, o.speed))
            .or_else(|| {
                let succ = *self.graph.successors(n_lane).first()?;
                let l = by_lane.get(&succ)?.first()?;
                let o = &self.agents[*l];
                Some((ml_length - s_t + o.s - o.params.length, o.speed))
            });
        let follower_t = list
            .iter()
            .map(|&j| &self.agents[j])
            .filter(|o| o.s <= s_t - own_length)
            .max_by(|x, y| x.s.total_cmp(&y.s))
            .map(|o| (o, s_t - own_length - o.s))
            .or_else(|| {
                self.graph
                    .predecessors(n_lane)
                    .iter()
                    .filter_map(|p| {
                        let pl = self.graph.lane(*p)?;
                        by_lane
                            .get(p)?
                            .iter()
                            .map(|&j| &self.agents[j])
                            .max_by(|x, y| x.s.total_cmp(&y.s))
                            .map(|o| (o, s_t - own_length + pl.length() - o.s))
                    })
                    .min_by(|x, y| x.1.total_cmp(&y.1))
            });
        (leader_t, follower_t)
    }

    fn compute_lane_change_bias(
        &self,
        a: &Agent,
        lane: &Lane,
        t: &Target,
        a_after: f64,
        a_now: f64,
    ) -> f64 {
        let (ml, n, mandatory) = (t.lane, t.n, t.mandatory);
        let heavy = a.params.class != crate::idm::VehicleClass::Car;
        let mut bias = match n.side {
            Side::Right if a_after >= a_now - 0.1 => {
                KEEP_RIGHT_BONUS + if heavy { HEAVY_KEEP_RIGHT_BONUS } else { 0.0 }
            }
            Side::Right => 0.0,
            Side::Left => -LEFT_PENALTY,
        };
        if a.params.class == crate::idm::VehicleClass::Truck && ml.rules.trucks != lane.rules.trucks
        {
            bias += if ml.rules.trucks {
                TRUCK_LANE_BONUS
            } else {
                -TRUCK_LANE_BONUS
            };
        }
        if mandatory {
            let to_go: f64 = lane.length() - crate::agent::STRATEGIC_LAST_M - a.s;
            let urgency: f64 = (1.0 - to_go / URGENCY_RANGE_M).clamp(0.0, 1.0);
            bias += STRATEGIC_BONUS + URGENCY_GROWTH * urgency;
        }
        bias
    }

    fn is_target_lane_allowed(
        &self,
        a: &Agent,
        ml: &Lane,
        lane: &Lane,
        n: &crate::neighbours::Neighbour,
        mandatory: bool,
    ) -> bool {
        let own = &a.params;
        ml.kind == lane.kind
            && (!ml.junction || mandatory)
            && !ml.rules.density.closed_for(usize::from(own.density_group))
            && ml.rules.allows(own.class)
            && ml.signal.is_none()
            && !(n.side == Side::Left && (lane.rules.no_overtaking || ml.rules.no_overtaking))
    }

    fn is_target_spot_free(
        &self,
        a: &Agent,
        t: &Target,
        s_t: f64,
        gate: f64,
        list: &[usize],
    ) -> bool {
        let (ml, n_lane, mandatory) = (t.lane, t.n.lane, t.mandatory);
        if s_t > ml.length() - 5.0
            || (self.graph.successors(n_lane).is_empty() && ml.length() - s_t < 40.0)
        {
            return false;
        }
        let inside = |l: LaneId, s: f64| {
            self.graph
                .conflicts(l)
                .iter()
                .any(|c| c.enter - 1.0 < s && s - a.params.length < c.exit + 1.0)
        };
        if inside(n_lane, s_t) {
            return false;
        }
        if !mandatory {
            let mut ahead = vec![(n_lane, -s_t)];
            let rest = ml.length() - s_t;
            ahead.extend(self.graph.successors(n_lane).iter().map(|&l| (l, rest)));
            if self.junction_within(&ahead, gate) {
                return false;
            }
        }
        let beside = list.iter().any(|&j| {
            let o = &self.agents[j];
            (o.s > s_t - a.params.length - SIDE_PAD && o.s - o.params.length < s_t + SIDE_PAD)
                || o.changing_lane() && (o.s - s_t).abs() < 40.0
        });
        !beside
    }

    fn eval_neighbour_candidate(
        &self,
        i: usize,
        n: &crate::neighbours::Neighbour,
        ctx: &EvalCtx,
    ) -> Option<Choice> {
        let EvalCtx {
            only,
            gate,
            a_now,
            a_o_diff,
            by_lane,
        } = *ctx;
        let a = &self.agents[i];
        let own = &a.params;
        let lane = self.graph.lane(a.lane)?;
        let mandatory = a.strategic.filter(|c| c.from == a.lane);
        if only.is_some_and(|s| s != n.side) || mandatory.is_some_and(|c| c.via != n.lane) {
            return None;
        }
        let ml = self.graph.lane(n.lane)?;
        if !self.is_target_lane_allowed(a, ml, lane, n, mandatory.is_some()) {
            return None;
        }
        let target = Target {
            n,
            lane: ml,
            mandatory: mandatory.is_some(),
        };
        let (s_t, sep) = n.at(a.s)?;
        let list: &[usize] = by_lane.get(&n.lane).map_or(&[], Vec::as_slice);
        if !self.is_target_spot_free(a, &target, s_t, gate, list) {
            return None;
        }

        let (leader_t, follower_t) =
            self.find_target_leader_follower(n.lane, ml.length(), s_t, own.length, list, by_lane);
        let (a_after, a_f_before, a_f_after) =
            self.check_target_safety(a, n.lane, leader_t, follower_t)?;
        let bias = self.compute_lane_change_bias(a, lane, &target, a_after, a_now);
        let total = (a_after - a_now) + POLITENESS * ((a_f_after - a_f_before) + a_o_diff) + bias;
        let worthwhile = if only.is_some() {
            total > 0.0
        } else {
            total > THRESHOLD
        };
        if worthwhile {
            Some(Choice {
                side: n.side,
                target: n.lane,
                s_target: s_t,
                separation: sep,
                total,
            })
        } else {
            None
        }
    }

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
    fn junction_within(&self, look: &[(LaneId, f64)], gate: f64) -> bool {
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
