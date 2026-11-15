//! The time step of the body on its springs (heave, pitch and roll).

use super::body::BodySuspension;
use super::{BODY_SUBSTEP_S, BUMP_STOP_STIFFNESS_N_M, BUMP_STOP_TRAVEL_M, GRAVITY};

impl BodySuspension {
    /// Advances `dt` seconds. `a_long` is the longitudinal acceleration (m/s², forward positive),
    /// `a_lat` the lateral one (m/s², towards the right positive); `ground_m` the height of the
    /// ground under each wheel against the plane the body follows (m, same order as
    /// [`BodySuspension::wheels`]; missing entries are 0).
    pub fn step(&mut self, dt: f32, a_long: f32, a_lat: f32, ground_m: &[f32]) {
        if dt <= 0.0 {
            return;
        }
        let n = (dt / BODY_SUBSTEP_S).ceil().max(1.0) as usize;
        let h = dt / n as f32;
        // The people on board add mass and inertia (their weight sits at the wheels' places).
        let (mut m, mut ip, mut ir) = (
            self.mass_kg,
            self.pitch_inertia_kg_m2,
            self.roll_inertia_kg_m2,
        );
        for w in &self.wheels {
            let pm = w.payload_n / GRAVITY;
            m += pm;
            ip += pm * w.x_m * w.x_m;
            ir += pm * w.y_m * w.y_m;
        }
        // The d'Alembert forces of the acceleration act at the centre of gravity: speeding up
        // lifts the nose, turning right rolls the body to the left (right side up).
        let m_pitch_ext = m * a_long * self.cg_height_m;
        let m_roll_ext = -m * a_lat * self.cg_height_m;
        let mut heave_acc_sum = 0.0f32;
        for _ in 0..n {
            let (mut fz, mut mp, mut mr) = (0.0f32, 0.0f32, 0.0f32);
            for (i, w) in self.wheels.iter_mut().enumerate() {
                let g = ground_m.get(i).copied().unwrap_or(0.0);
                // Height of the body above this wheel's rest position, then the compression.
                let body = self.heave_m + w.x_m * self.pitch_rad - w.y_m * self.roll_rad;
                let body_rate = self.heave_rate_m_s + w.x_m * self.pitch_rate_rad_s
                    - w.y_m * self.roll_rate_rad_s;
                let compression = -body + g;
                let rate = -body_rate;
                // The factor scales the whole spring, the rest load included: the body rests
                // lower on a softer spring (kneeling, `Axle_Springfactor` 0.55) and higher on a
                // stiffer one (raising to 2), where scaling only the compression would change
                // the stiffness and never the height.
                let mut force = w.spring_n_m * w.spring_factor * compression
                    + (w.spring_factor - 1.0) * w.static_load_n
                    + w.damping_n_s_m * rate;
                if compression > BUMP_STOP_TRAVEL_M {
                    force += BUMP_STOP_STIFFNESS_N_M * (compression - BUMP_STOP_TRAVEL_M);
                }
                // A wheel cannot pull: at most it loses all its load.
                let load = (w.static_load_n + force).max(0.0);
                let delta = load - w.static_load_n;
                w.compression_m = compression;
                w.compression_rate_m_s = rate;
                w.load_n = load;
                // The spring's deviation lifts the body, the payload's weight pulls it down.
                let net = delta - w.payload_n;
                fz += net;
                mp += w.x_m * net;
                mr -= w.y_m * net;
            }
            let heave_acc = fz / m;
            heave_acc_sum += heave_acc;
            let pitch_acc = (mp + m_pitch_ext) / ip;
            let roll_acc = (mr + m_roll_ext) / ir;
            self.heave_rate_m_s += heave_acc * h;
            self.pitch_rate_rad_s += pitch_acc * h;
            self.roll_rate_rad_s += roll_acc * h;
            self.heave_m += self.heave_rate_m_s * h;
            self.pitch_rad += self.pitch_rate_rad_s * h;
            self.roll_rad += self.roll_rate_rad_s * h;
        }
        self.vertical_acc_m_s2 = heave_acc_sum / n as f32;
    }
}
