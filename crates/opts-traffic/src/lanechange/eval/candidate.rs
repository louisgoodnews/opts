//! Evaluation of one neighbour lane as a candidate for a lane change.

use super::*;

impl TrafficWorld {
    pub(super) fn is_target_spot_free(
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

    pub(super) fn eval_neighbour_candidate(
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
}
