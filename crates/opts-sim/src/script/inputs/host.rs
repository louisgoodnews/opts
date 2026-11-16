//! Filling the host inputs: the per-wheel and per-door setters and the lookups the feed reads.

use super::*;

impl HostInputs {
    /// The value of a scalar row.
    #[must_use]
    pub fn value(&self, row: &InputRow) -> f32 {
        let b = |v: bool| f32::from(u8::from(v));
        match (row.name, row.kind) {
            ("velocity", _) => self.velocity_km_h,
            ("velocity_ground", _) => self.velocity_ground_km_h,
            ("n_wheel", _) => self.n_wheel_rpm,
            ("a_trans_x", _) => self.a_trans[0],
            ("a_trans_y", _) => self.a_trans[1],
            ("a_trans_z", _) => self.a_trans[2],
            ("kmcounter_km", _) => (self.odometer_m / 1000.0).floor() as f32,
            ("kmcounter_m", _) => self.odometer_m.rem_euclid(1000.0) as f32,
            ("tank_percent", _) => self.tank_percent,
            ("throttle", _) => self.throttle,
            ("brake", _) => self.brake,
            ("clutch", _) => self.clutch,
            ("ai", _) => b(self.ai),
            ("ai_engine", _) => self.ai_engine,
            ("ai_target_index", _) => self.ai_target_index,
            ("schedule_active", _) => b(self.schedule_active),
            ("humans_count", _) => self.humans_count as f32,
            ("envir_brightness", _) => self.envir_brightness,
            ("streetcond", _) => self.street_cond,
            ("dirtrate", _) => self.dirt_rate,
            ("wearlifespan", _) => self.wearlifespan,
            ("preciprate", _) => self.precip_rate,
            ("preciptype", _) => self.precip_type,
            ("time", _) => self.time_of_day_s,
            ("day", _) => self.day as f32,
            ("month", _) => self.month as f32,
            ("year", _) => self.year as f32,
            ("dayofyear", _) => self.day_of_year as f32,
            ("sunalt", _) => self.sun_alt_deg,
            ("pause", _) => b(self.pause),
            ("weather_temperature", _) => self.temperature_c,
            ("weather_abshum", _) => self.abs_hum,
            ("autoclutch", _) => b(self.auto_clutch),
            ("coll_energy", _) => self.coll_energy,
            ("coll_pos_x", _) => self.coll_pos[0],
            ("coll_pos_y", _) => self.coll_pos[1],
            ("coll_pos_z", _) => self.coll_pos[2],
            _ => 0.0,
        }
    }

    /// The value of a wheel family member (0 beyond the vehicle's axles).
    #[must_use]
    pub fn wheel(&self, family: WheelFamily, axle: usize, right: bool) -> f32 {
        let Some(a) = self.axles.get(axle) else {
            return 0.0;
        };
        let w = if right { &a.right } else { &a.left };
        match family {
            WheelFamily::RotationSpeed => w.speed_rpm,
            WheelFamily::Rotation => w.rotation_rad,
            WheelFamily::Steering => w.steering_rad,
            WheelFamily::Suspension => w.suspension_m,
        }
    }
}
