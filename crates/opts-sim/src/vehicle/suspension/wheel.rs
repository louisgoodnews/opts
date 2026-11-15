//! One wheel's spring and damper and its travel as OMSI reports it.

use serde::{Deserialize, Serialize};

fn one() -> f32 {
    1.0
}

/// Lowest and highest factor on a spring the physics accepts (a script bug must not turn the
/// body into a rock or a jelly).
pub const SPRING_FACTOR_RANGE: (f32, f32) = (0.2, 5.0);

/// One wheel's spring and damper under the body.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WheelSuspension {
    /// Position under the body: longitudinal (m, forward positive) and lateral (m, right positive).
    pub x_m: f32,
    /// Lateral position (m).
    pub y_m: f32,
    /// Spring rate of this wheel (N/m).
    pub spring_n_m: f32,
    /// Damping of this wheel (N·s/m).
    pub damping_n_s_m: f32,
    /// Factor on the spring rate the scripts write (`Axle_Springfactor_<n>_<l|r>`, air
    /// suspension and kneeling); 1.0 is the file's spring.
    #[serde(default = "one")]
    pub spring_factor: f32,
    /// Load at rest (N).
    pub static_load_n: f32,
    /// Spring compression beyond the rest position (m, compression positive).
    pub compression_m: f32,
    /// Rate of the compression (m/s).
    pub compression_rate_m_s: f32,
    /// Vertical load on the ground now (N, zero when the wheel has lifted).
    pub load_n: f32,
    /// The wheel's share of the weight of the people on board (N): a downward force on the body
    /// at the wheel's position (M-0124); the body sinks until the springs carry it.
    #[serde(default)]
    pub payload_n: f32,
}

impl WheelSuspension {
    /// The wheel's travel as OMSI's `Axle_Suspension_<n>_<l|r>` gives it (m): the extension of
    /// the spring from its unloaded length, so negative under load and growing when the body
    /// rises. Read from the scripts' own use: the air-suspension levelling opens the fill valve
    /// while the value is below the set point (`bremse_level_setpoint` -0.105 in the MAN and
    /// Valiant brake scripts, "the body stands lower than the set point") and the drain valve
    /// while it is above, and about -0.1 m is the static deflection of a city bus wheel on its
    /// `achse_feder` (Valiant 7700 front: 24.7 kN on 284.6 kN/m, 0.087 m). The spring of the
    /// body model is linear in the factor (`f · k · (rest + compression)`), so the unloaded
    /// length is `static_load / k` above the rest position whatever the factor.
    #[must_use]
    pub fn omsi_travel_m(&self) -> f32 {
        -(self.static_load_n / self.spring_n_m.max(1.0) + self.compression_m)
    }
}
