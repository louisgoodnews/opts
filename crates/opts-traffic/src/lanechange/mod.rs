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

mod eval;

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
            if self.agents[i].foreign {
                continue;
            }
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
                        self.stats.lane_change_aborts += 1;
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
        if a.strategic.is_some_and(|s| s.via == c.target) {
            a.strategic = None;
        } else {
            a.strategic = None;
            a.plan.clear();
        }
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
}
