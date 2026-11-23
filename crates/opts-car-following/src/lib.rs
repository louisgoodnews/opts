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
    /// How rowdy the driver is, `0` (careful) to `1` (reckless): the more, the more yellow lights
    /// are passed ([`IdmParams::yellow_decel`]).
    pub rowdy: f64,
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
            rowdy: 0.0,
        }
    }
}

impl IdmParams {
    /// The braking (m/s²) the driver accepts to stop for a yellow light: a careful driver stops
    /// whenever 3 m/s² suffices; the rowdier, the less braking they accept (down to 30 % of that),
    /// so they pass more yellow lights.
    #[must_use]
    pub fn yellow_decel(&self, comfortable: f64) -> f64 {
        comfortable * (1.0 - 0.7 * self.rowdy.clamp(0.0, 1.0))
    }

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
    /// look-ahead limits; `TrafficWorld` in `opts-traffic` computes it) and an
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
mod tests;
