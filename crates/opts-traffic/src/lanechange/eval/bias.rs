//! Incentive and admission of a lane change: the bias towards keeping right and the rules of the target lane.

use super::*;

impl TrafficWorld {
    pub(super) fn compute_lane_change_bias(
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

    pub(super) fn is_target_lane_allowed(
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
}
