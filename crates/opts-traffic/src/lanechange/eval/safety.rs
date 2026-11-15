//! Target-lane safety: the new follower's and the changer's braking, and the gaps to the neighbours.

use super::*;

impl TrafficWorld {
    pub(super) fn check_target_safety(
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

    pub(super) fn find_target_leader_follower<'a>(
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
}
