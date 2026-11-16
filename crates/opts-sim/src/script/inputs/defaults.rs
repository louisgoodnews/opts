//! The default host inputs: a standing vehicle at rest with its wheels and doors shut.

use super::{HostInputs, DOORS, WEARLIFESPAN_NO_BREAKDOWNS};

impl Default for HostInputs {
    fn default() -> Self {
        Self {
            velocity_km_h: 0.0,
            velocity_ground_km_h: 0.0,
            n_wheel_rpm: 0.0,
            a_trans: [0.0; 3],
            odometer_m: 0.0,
            tank_percent: 100.0,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            ai: false,
            ai_engine: 0.0,
            ai_target_index: -1.0,
            schedule_active: false,
            humans_count: 0,
            pax_entry_req: [false; DOORS],
            pax_exit_req: [false; DOORS],
            envir_brightness: 1.0,
            street_cond: 0.0,
            dirt_rate: 0.0,
            wearlifespan: WEARLIFESPAN_NO_BREAKDOWNS,
            precip_rate: 0.0,
            precip_type: 0.0,
            time_of_day_s: 12.0 * 3600.0,
            day: 29,
            month: 6,
            year: 2026,
            day_of_year: 180,
            sun_alt_deg: 60.0,
            pause: false,
            temperature_c: 15.0,
            abs_hum: 8.0,
            auto_clutch: true,
            coll_energy: 0.0,
            coll_pos: [0.0; 3],
            axles: Vec::new(),
        }
    }
}
