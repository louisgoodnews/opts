//! The Intelligent Driver Model (Treiber, Hennecke, Helbing 2000), the car-following law used for
//! AI vehicles.
//!
//! ```text
//! a = A · [ 1 − (v / v0)^δ − (s* / s)² ]        s* = s0 + v·T + v·Δv / (2·√(A·B))
//! ```
//!
//! `s` is the net gap to the leader and `Δv` the approach speed (own minus leader speed). The
//! defaults are the literature's city values, **not** calibrated against OMSI's AI ❓
//! (`docs/src/architecture/ai-traffic.md`, section "Calibration").

/// What kind of vehicle it is, for lane restrictions (`bus`, `trucks`, `no_cars` rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VehicleClass {
    /// A passenger car (and van, taxi, motorcycle).
    #[default]
    Car,
    /// A lorry or other commercial vehicle.
    Truck,
    /// A bus or tram-like public transport vehicle.
    Bus,
}

/// Speed (m/s) on a lane without a speed limit (the town default, 50 km/h).
pub const OPEN_ROAD_SPEED: f64 = 50.0 / 3.6;

/// Parameters of one driver/vehicle pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdmParams {
    /// Highest speed this vehicle and driver want anywhere, m/s (the vehicle's top speed). The
    /// desired speed on a lane is [`Self::lane_speed`]: the lane's limit times
    /// [`Self::speed_factor`], never above this.
    pub desired_speed: f64,
    /// The driver's factor on speed limits (`1.0` drives at the limit, `1.1` ten percent above).
    pub speed_factor: f64,
    /// Lateral acceleration (m/s²) the driver accepts in curves: `v ≤ √(a_lat · |R|)`.
    pub lat_accel: f64,
    /// Desired time headway `T` in seconds.
    pub time_headway: f64,
    /// Minimum gap `s0` in metres, kept at standstill.
    pub min_gap: f64,
    /// Maximum acceleration `A` in m/s².
    pub max_accel: f64,
    /// Comfortable deceleration `B` in m/s².
    pub comfort_decel: f64,
    /// Acceleration exponent `δ`.
    pub delta: f64,
    /// Vehicle length in metres (front bumper to rear bumper).
    pub length: f64,
    /// Distance (m) from the front bumper back to the front axle, which follows the lane.
    pub front_overhang: f64,
    /// Distance (m) from the front axle to the (rearmost) rear axle; the rear axle is dragged
    /// behind the front one (a tractrix), which gives the body its heading.
    pub wheelbase: f64,
    /// Hardest braking the model may ask for, m/s² (keeps the law crash-free yet bounded).
    pub max_decel: f64,
    /// Vehicle class for lane restrictions.
    pub class: VehicleClass,
    /// The vehicle's group: its 0-based index in `unsched_vehgroups.txt`, which the `flag` of a
    /// `[rule] trafficdensity` names. A lane whose rule for this group is `0` is not entered by
    /// lane changes.
    pub density_group: u8,
}

impl Default for IdmParams {
    /// A passenger car in town: the limit (50 km/h on an open road), 1.5 s headway, 2 m minimum
    /// gap, 130 km/h top speed, 2.5 m/s² in curves.
    fn default() -> Self {
        Self {
            desired_speed: 130.0 / 3.6,
            speed_factor: 1.0,
            lat_accel: 2.5,
            time_headway: 1.5,
            min_gap: 2.0,
            max_accel: 1.4,
            comfort_decel: 2.0,
            delta: 4.0,
            length: 4.4,
            front_overhang: 0.85,
            wheelbase: 0.6 * 4.4,
            max_decel: 9.0,
            class: VehicleClass::Car,
            density_group: 0,
        }
    }
}

impl IdmParams {
    /// A city bus or lorry: slower to accelerate, gentler braking, longer, 80 km/h top speed and
    /// 2.0 m/s² in curves.
    #[must_use]
    pub fn heavy() -> Self {
        Self {
            desired_speed: 80.0 / 3.6,
            lat_accel: 2.0,
            max_accel: 0.8,
            comfort_decel: 1.5,
            length: 12.0,
            wheelbase: 0.6 * 12.0,
            class: VehicleClass::Truck,
            ..Self::default()
        }
    }

    /// Desired speed (m/s) on a lane with speed limit `limit` (`None`: no limit, `open_road`
    /// applies): `min(top speed, factor · limit)`.
    #[must_use]
    pub fn lane_speed(&self, limit: Option<f64>, open_road: f64) -> f64 {
        let limit = limit.unwrap_or(open_road);
        self.desired_speed.min(self.speed_factor * limit).max(0.1)
    }

    /// Highest speed (m/s) on a circular arc of radius `radius` (`0` = straight: unbounded).
    #[must_use]
    pub fn curve_speed(&self, radius: f64) -> f64 {
        if radius.abs() < 1e-3 || !radius.is_finite() {
            f64::INFINITY
        } else {
            (self.lat_accel.max(0.1) * radius.abs()).sqrt()
        }
    }

    /// Desired dynamic gap `s*` at speed `v` and approach speed `dv`.
    #[must_use]
    pub fn desired_gap(&self, v: f64, dv: f64) -> f64 {
        let dynamic = v * dv / (2.0 * (self.max_accel * self.comfort_decel).sqrt());
        (self.min_gap + v * self.time_headway + dynamic).max(self.min_gap)
    }

    /// Acceleration for speed `v`, a free-road speed cap `v_cap` (the lane speed, curve and
    /// look-ahead limits; [`crate::TrafficWorld`] computes it) and an
    /// optional obstacle `(net gap, speed)` ahead. A standing obstacle such as a stop line has
    /// speed `0`.
    #[must_use]
    pub fn acceleration(&self, v: f64, v_cap: f64, ahead: Option<(f64, f64)>) -> f64 {
        let v0 = self.desired_speed.min(v_cap).max(0.1);
        let free = 1.0 - (v / v0).powf(self.delta);
        let interaction = ahead.map_or(0.0, |(gap, v_lead)| {
            let s = gap.max(0.01);
            (self.desired_gap(v, v - v_lead) / s).powi(2)
        });
        (self.max_accel * (free - interaction)).max(-self.max_decel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_road_converges_to_the_desired_speed() {
        let p = IdmParams::default();
        let mut v = 0.0;
        for _ in 0..6000 {
            v += p.acceleration(v, f64::INFINITY, None) * 0.05;
        }
        assert!((v - p.desired_speed).abs() < 0.05, "v = {v}");
    }

    #[test]
    fn speed_limit_caps_the_free_road_speed() {
        let p = IdmParams::default();
        assert!(p.acceleration(5.0, 5.0, None).abs() < 1e-9);
        assert!(p.acceleration(8.0, 5.0, None) < 0.0);
    }

    #[test]
    fn closing_on_a_standing_leader_brakes_and_stops_short() {
        let p = IdmParams::default();
        let (mut v, mut x, leader_rear) = (p.desired_speed, 0.0, 120.0);
        for _ in 0..4000 {
            let a = p.acceleration(v, f64::INFINITY, Some((leader_rear - x, 0.0)));
            let nv = (v + a * 0.05).max(0.0);
            x += (v + nv) / 2.0 * 0.05;
            v = nv;
        }
        let gap = leader_rear - x;
        assert!(v < 0.05, "still moving at {v}");
        assert!(
            gap > 0.5 && gap < 4.0,
            "stopped {gap} m behind the obstacle"
        );
    }

    #[test]
    fn a_leader_at_the_same_speed_is_followed_at_the_desired_headway() {
        let p = IdmParams::default();
        let v = 10.0;
        // Equilibrium gap: s0 + vT, up to the free-road term.
        let s_eq =
            (p.min_gap + v * p.time_headway) / (1.0 - (v / p.desired_speed).powf(p.delta)).sqrt();
        assert!(p.acceleration(v, f64::INFINITY, Some((s_eq, v))).abs() < 1e-9);
    }

    #[test]
    fn lane_speed_is_the_factor_on_the_limit_below_the_top_speed() {
        let p = IdmParams {
            speed_factor: 1.1,
            ..IdmParams::default()
        };
        assert!((p.lane_speed(Some(10.0), OPEN_ROAD_SPEED) - 11.0).abs() < 1e-9);
        assert!((p.lane_speed(None, OPEN_ROAD_SPEED) - 1.1 * OPEN_ROAD_SPEED).abs() < 1e-9);
        let slow = IdmParams {
            desired_speed: 5.0,
            ..p
        };
        assert_eq!(slow.lane_speed(Some(30.0), OPEN_ROAD_SPEED), 5.0);
        // √(2.5 · 12) for a car, √(2.0 · 12) for a lorry; straight is unbounded.
        assert!((p.curve_speed(-12.0) - 30f64.sqrt()).abs() < 1e-9);
        assert!((IdmParams::heavy().curve_speed(12.0) - 24f64.sqrt()).abs() < 1e-9);
        assert!(p.curve_speed(0.0).is_infinite());
    }

    #[test]
    fn braking_is_bounded() {
        let p = IdmParams::default();
        assert_eq!(
            p.acceleration(13.0, f64::INFINITY, Some((0.5, 0.0))),
            -p.max_decel
        );
    }
}
