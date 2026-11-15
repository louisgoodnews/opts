//! Speed caps of the world: the limit of a lane and the speed a vehicle wants there.

use super::*;

impl TrafficWorld {
    /// Sets the limit (m/s) assumed on lanes without a `speedlimit` rule (default 50 km/h).
    pub fn set_open_road_speed(&mut self, speed: f64) {
        self.open_road_speed = speed.max(0.1);
    }

    /// The speed (m/s) a vehicle with `params` wants on `lane` by itself: the lane speed of the
    /// driver ([`IdmParams::lane_speed`]) and the curve limit of the lane's arc.
    #[must_use]
    pub fn lane_cap(&self, params: &IdmParams, lane: LaneId) -> f64 {
        self.graph.lane(lane).map_or(f64::INFINITY, |l| {
            params
                .lane_speed(l.speed_limit, self.open_road_speed)
                .min(params.curve_speed(l.geometry.radius))
        })
    }

    /// The free-road speed cap of agent `i` (the IDM's desired speed) and the braking it needs
    /// for lower lane speeds ahead (m/s², `≤ 0`, or `+∞` when none).
    ///
    /// The cap is that of its own lane, and of every planned lane within its comfortable braking
    /// distance plus [`SPEED_LOOK_EXTRA_M`], reduced by what it can shed on the way
    /// (`√(v_lim² + 2·b·d)`). Because the IDM's free term lags behind a falling cap, a vehicle
    /// faster than a lane's speed brakes explicitly once it needs at least half its comfortable
    /// deceleration to reach it at the lane's start: it slows before a bend or a lower limit
    /// instead of in it.
    pub(crate) fn speed_cap(&self, i: usize) -> (f64, f64) {
        let a = &self.agents[i];
        let b = a.params.comfort_decel.max(0.1);
        let v = a.speed;
        let reach = v * v / (2.0 * b) + SPEED_LOOK_EXTRA_M;
        let own = self.lane_cap(&a.params, a.lane);
        let mut cap = own;
        let mut brake = if v > own {
            -(v - own) / SPEED_TAU_S
        } else {
            f64::INFINITY
        };
        let Some(lane) = self.graph.lane(a.lane) else {
            return (cap, brake);
        };
        let mut d = lane.length() - a.s;
        for &p in &a.plan {
            if d > reach {
                break;
            }
            let v_lim = self.lane_cap(&a.params, p);
            cap = cap.min((v_lim * v_lim + 2.0 * b * d).sqrt());
            if v > v_lim {
                let need = (v * v - v_lim * v_lim) / (2.0 * (d - SPEED_MARGIN_M).max(0.5));
                if need > 0.5 * b {
                    brake = brake.min(-need);
                }
            }
            d += self.graph.lane(p).map_or(0.0, |l| l.length());
        }
        (cap, brake)
    }
}
