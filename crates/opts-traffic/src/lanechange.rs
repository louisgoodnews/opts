//! Lane changes: decided like MOBIL (Kesting, Treiber, Helbing 2007) but with a stricter notion of
//! *priority* and *gentleness*.
//!
//! OMSI's oldest complaint is that AI cars brake hard when they change lanes. Here a lane change
//! can only start when it needs no hard braking from anyone:
//!
//! * **The lane one changes onto has priority** (German rule: the changing vehicle must not
//!   endanger others, StVO §7 (5)). The new follower must not have to decelerate harder than
//!   [`B_SAFE_FOLLOWER`] with the changer in front of it, nobody may be beside the changer's spot,
//!   and no other vehicle may be changing into the same lane there.
//! * **The changer itself** must not need more than [`B_SAFE_SELF`] of braking behind its new leader.
//!   Its speed is the speed of its old lane — a faster follower in the target lane simply means no
//!   gap yet, so the changer waits instead of cutting in.
//! * The change is **announced**: the indicator is on for [`ANNOUNCE_S`] seconds, the situation is
//!   checked again and only then is the vehicle moved. The car is placed on the new lane at once
//!   (so followers react to it immediately and gently) and glides sideways over
//!   [`crate::LANE_CHANGE_DURATION_S`] seconds.
//! * The incentive is the gain in own acceleration, plus a small politeness share of the others'
//!   change, plus a bonus for moving right (*Rechtsfahrgebot*) and a penalty for moving left;
//!   `overtaking_prohib` forbids moves to the left. Trucks and buses keep right more strongly,
//!   and trucks prefer lanes with a `trucks` rule.
//! * **Strategic changes:** when the route continues from a successor of a neighbour lane
//!   ([`crate::agent::Strategic`]), the change to that neighbour is mandatory: it is the only one
//!   considered on that lane, junctions ahead do not block it, and an urgency bonus grows as the
//!   lane's end comes closer (the vehicle waits at the end otherwise).
//!
//! No lane changes inside a conflict zone or on a junction object's lane, nor within
//! `max(25 m, 2.5 s · v)` of a conflict zone or a light's stop line ahead on the own or the
//! target lane, and not onto a junction object's lane (zones already passed do not count); none below 3 m/s (1 m/s for a
//! strategic change) or within [`COOLDOWN_S`] seconds of the previous one.

use std::collections::HashMap;

use crate::agent::TrafficEvent;
use crate::graph::LaneId;
use crate::neighbours::Side;
use crate::TrafficWorld;

/// Seconds between evaluations of one agent.
const CHECK_INTERVAL_S: f64 = 1.0;
/// Seconds the indicator is on before the vehicle moves over.
pub const ANNOUNCE_S: f64 = 3.0;
/// Seconds without a new lane change after one.
pub const COOLDOWN_S: f64 = 8.0;
/// Slowest speed (m/s) at which vehicles change lanes.
const MIN_SPEED: f64 = 3.0;
/// Slowest speed (m/s) at which a strategic change is made.
const MIN_SPEED_STRATEGIC: f64 = 1.0;
/// Hardest braking (m/s²) the new follower may need.
pub const B_SAFE_FOLLOWER: f64 = 1.5;
/// Hardest braking (m/s²) the changer may need behind its new leader.
pub const B_SAFE_SELF: f64 = 1.0;
/// Weight of the other drivers' change of acceleration.
const POLITENESS: f64 = 0.2;
/// Gain (m/s²) a change must bring.
const THRESHOLD: f64 = 0.25;
/// Bonus (m/s²) for moving right when that is not worse.
const KEEP_RIGHT_BONUS: f64 = 0.3;
/// Penalty (m/s²) for moving left.
const LEFT_PENALTY: f64 = 0.1;
/// Free length (m) required beside the target spot, front and back.
const SIDE_PAD: f64 = 2.0;
/// Smallest distance (m) to a junction ahead that still allows changing lanes …
const GATE_MIN_M: f64 = 25.0;
/// … or this many seconds of travel, whichever is more.
const GATE_TIME_S: f64 = 2.5;
/// Extra keep-right bonus (m/s²) of trucks and buses.
const HEAVY_KEEP_RIGHT_BONUS: f64 = 0.3;
/// Bonus (m/s²) for a truck moving onto a `trucks` lane (and penalty for leaving one).
const TRUCK_LANE_BONUS: f64 = 0.4;
/// Base and growth (m/s²) of the incentive of a strategic change; it grows over the last
/// [`URGENCY_RANGE_M`] metres before the change point.
const STRATEGIC_BONUS: f64 = 0.5;
const URGENCY_GROWTH: f64 = 1.5;
const URGENCY_RANGE_M: f64 = 100.0;

/// A lane change that passed the checks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Choice {
    side: Side,
    target: LaneId,
    s_target: f64,
    separation: f64,
    total: f64,
}

impl TrafficWorld {
    /// Evaluates announcements, executes due lane changes and announces new ones. At most one
    /// agent moves per step. Returns whether an agent moved.
    pub(crate) fn lane_change_pass(&mut self, by_lane: &HashMap<LaneId, Vec<usize>>) -> bool {
        for i in 0..self.agents.len() {
            let (speed, busy, due, next_check, cooldown, side) = {
                let a = &self.agents[i];
                (
                    a.speed,
                    a.changing_lane(),
                    a.lc_due,
                    a.lc_next_check,
                    a.lc_cooldown_until,
                    a.lc_side,
                )
            };
            if busy {
                continue;
            }
            if let (Some(due), Some(side)) = (due, side) {
                if self.time < due {
                    continue;
                }
                match self.evaluate(i, Some(side), by_lane) {
                    Some(choice) => {
                        self.execute(i, choice);
                        return true;
                    }
                    None => {
                        let a = &mut self.agents[i];
                        a.lc_due = None;
                        a.lc_side = None;
                        a.lc_cooldown_until = self.time + 3.0;
                    }
                }
                continue;
            }
            let mandatory = self.agents[i]
                .strategic
                .is_some_and(|c| c.from == self.agents[i].lane);
            let min_speed = if mandatory {
                MIN_SPEED_STRATEGIC
            } else {
                MIN_SPEED
            };
            if speed < min_speed || self.time < next_check || self.time < cooldown {
                continue;
            }
            self.agents[i].lc_next_check = self.time + CHECK_INTERVAL_S;
            if let Some(choice) = self.evaluate(i, None, by_lane) {
                let a = &mut self.agents[i];
                a.lc_due = Some(self.time + ANNOUNCE_S);
                a.lc_side = Some(choice.side);
            }
        }
        false
    }

    fn execute(&mut self, i: usize, c: Choice) {
        let time = self.time;
        let a = &mut self.agents[i];
        a.lane = c.target;
        a.s = c.s_target;
        a.prev_lane = None;
        // The planned change keeps the route (it continues from the target); any other change
        // starts a new one.
        if a.strategic.is_some_and(|s| s.via == c.target) {
            a.strategic = None;
        } else {
            a.strategic = None;
            a.plan.clear();
        }
        // The car now belongs to the new lane but is still beside it, on the side it came from.
        let offset = match c.side {
            Side::Right => -c.separation,
            Side::Left => c.separation,
        };
        a.begin_lane_change(offset, c.side);
        a.lc_due = None;
        a.lc_cooldown_until = time + COOLDOWN_S;
        a.lc_next_check = time + CHECK_INTERVAL_S;
        let id = a.id;
        self.push_event(TrafficEvent::LaneChanged { id, side: c.side });
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
        // Never on a junction object's lane or inside a conflict zone.
        let inside = |l: LaneId, s: f64| {
            self.graph
                .conflicts(l)
                .iter()
                .any(|c| c.enter - 1.0 < s && s - own.length < c.exit + 1.0)
        };
        if lane.junction || inside(a.lane, a.s) {
            return None;
        }
        // Not within the gate of a junction ahead (a strategic change has to happen anyway).
        if mandatory.is_none() && self.junction_within(&look, gate) {
            return None;
        }
        let cap_now = self.lane_cap(own, a.lane);
        let leader_now = self.leader(i, &look, by_lane);
        let a_now = own.acceleration(a.speed, cap_now, leader_now);
        // The vehicle behind me on my lane: its comfort changes when I leave.
        let old_follower = by_lane.get(&a.lane).and_then(|l| {
            let pos = l.iter().position(|&j| j == i)?;
            pos.checked_sub(1).map(|p| l[p])
        });
        let (a_o_before, a_o_after) = old_follower.map_or((0.0, 0.0), |j| {
            let o = &self.agents[j];
            let cap = self.lane_cap(&o.params, a.lane);
            let gap_me = a.s - own.length - o.s;
            let before = o.params.acceleration(o.speed, cap, Some((gap_me, a.speed)));
            let after_ahead = leader_now.map(|(g, v)| (g + own.length + gap_me, v));
            (before, o.params.acceleration(o.speed, cap, after_ahead))
        });
        let mut best: Option<Choice> = None;
        for n in self.graph.neighbours(a.lane) {
            if only.is_some_and(|s| s != n.side) || mandatory.is_some_and(|c| c.via != n.lane) {
                continue;
            }
            let ml = self.graph.lane(n.lane)?;
            let allowed = ml.kind == lane.kind
                && (!ml.junction || mandatory.is_some())
                && ml.rules.density != Some(0.0)
                && ml.rules.allows(own.class)
                && ml.signal.is_none()
                && !(n.side == Side::Left && (lane.rules.no_overtaking || ml.rules.no_overtaking));
            if !allowed {
                continue;
            }
            let Some((s_t, sep)) = n.at(a.s) else {
                continue;
            };
            if s_t > ml.length() - 5.0
                || (self.graph.successors(n.lane).is_empty() && ml.length() - s_t < 40.0)
            {
                continue;
            }
            // No conflict zone of the target lane at the spot, nor (unless the change is
            // mandatory) a junction within the gate ahead on it.
            if inside(n.lane, s_t) {
                continue;
            }
            if mandatory.is_none() {
                let mut ahead = vec![(n.lane, -s_t)];
                let rest = ml.length() - s_t;
                ahead.extend(self.graph.successors(n.lane).iter().map(|&l| (l, rest)));
                if self.junction_within(&ahead, gate) {
                    continue;
                }
            }
            let cap_t = self.lane_cap(own, n.lane);
            let list: &[usize] = by_lane.get(&n.lane).map_or(&[], Vec::as_slice);
            // Someone beside me, or changing into the same spot: no.
            let beside = list.iter().any(|&j| {
                let o = &self.agents[j];
                (o.s > s_t - own.length - SIDE_PAD && o.s - o.params.length < s_t + SIDE_PAD)
                    || o.changing_lane() && (o.s - s_t).abs() < 40.0
            });
            if beside {
                continue;
            }
            // New leader (this lane, else the first successor) and new follower (this lane, else
            // a predecessor).
            let leader_t = list
                .iter()
                .map(|&j| &self.agents[j])
                .filter(|o| o.s - o.params.length >= s_t)
                .min_by(|x, y| x.s.total_cmp(&y.s))
                .map(|o| (o.s - o.params.length - s_t, o.speed))
                .or_else(|| {
                    let succ = *self.graph.successors(n.lane).first()?;
                    let l = by_lane.get(&succ)?.first()?;
                    let o = &self.agents[*l];
                    Some((ml.length() - s_t + o.s - o.params.length, o.speed))
                });
            let follower_t = list
                .iter()
                .map(|&j| &self.agents[j])
                .filter(|o| o.s <= s_t - own.length)
                .max_by(|x, y| x.s.total_cmp(&y.s))
                .map(|o| (o, s_t - own.length - o.s))
                .or_else(|| {
                    self.graph
                        .predecessors(n.lane)
                        .iter()
                        .filter_map(|p| {
                            let pl = self.graph.lane(*p)?;
                            by_lane
                                .get(p)?
                                .iter()
                                .map(|&j| &self.agents[j])
                                .max_by(|x, y| x.s.total_cmp(&y.s))
                                .map(|o| (o, s_t - own.length + pl.length() - o.s))
                        })
                        .min_by(|x, y| x.1.total_cmp(&y.1))
                });
            // Safety: nobody brakes hard because of me.
            let (a_f_before, a_f_after) = match follower_t {
                Some((f, gap)) => {
                    if gap < f.params.min_gap + 0.5 {
                        continue;
                    }
                    let after = f.params.acceleration(
                        f.speed,
                        self.lane_cap(&f.params, n.lane),
                        Some((gap, a.speed)),
                    );
                    if after < -B_SAFE_FOLLOWER {
                        continue;
                    }
                    let lead_gap = leader_t.map(|(g, v)| (g + own.length + gap, v));
                    (
                        f.params
                            .acceleration(f.speed, self.lane_cap(&f.params, n.lane), lead_gap),
                        after,
                    )
                }
                None => (0.0, 0.0),
            };
            if leader_t.is_some_and(|(g, _)| g < own.min_gap + 1.0) {
                continue;
            }
            let a_after = own.acceleration(a.speed, cap_t, leader_t);
            if a_after < -B_SAFE_SELF {
                continue;
            }
            let heavy = own.class != crate::idm::VehicleClass::Car;
            let mut bias = match n.side {
                Side::Right if a_after >= a_now - 0.1 => {
                    KEEP_RIGHT_BONUS + if heavy { HEAVY_KEEP_RIGHT_BONUS } else { 0.0 }
                }
                Side::Right => 0.0,
                Side::Left => -LEFT_PENALTY,
            };
            if own.class == crate::idm::VehicleClass::Truck && ml.rules.trucks != lane.rules.trucks
            {
                bias += if ml.rules.trucks {
                    TRUCK_LANE_BONUS
                } else {
                    -TRUCK_LANE_BONUS
                };
            }
            if mandatory.is_some() {
                let to_go = lane.length() - crate::agent::STRATEGIC_LAST_M - a.s;
                let urgency = (1.0 - to_go / URGENCY_RANGE_M).clamp(0.0, 1.0);
                bias += STRATEGIC_BONUS + URGENCY_GROWTH * urgency;
            }
            let total = (a_after - a_now)
                + POLITENESS * ((a_f_after - a_f_before) + (a_o_after - a_o_before))
                + bias;
            let worthwhile = if only.is_some() {
                total > 0.0
            } else {
                total > THRESHOLD
            };
            if worthwhile && best.is_none_or(|b| total > b.total) {
                best = Some(Choice {
                    side: n.side,
                    target: n.lane,
                    s_target: s_t,
                    separation: sep,
                    total,
                });
            }
        }
        best
    }
}

impl TrafficWorld {
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
