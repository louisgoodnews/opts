//! The body on its wheels: rest state, construction, payload and load sums.

use super::wheel::WheelSuspension;
use crate::vehicle::axle::AxleConfig;
use serde::{Deserialize, Serialize};

/// The body on its wheels: heave, pitch and roll under a spring and a damper per wheel, driven by
/// the longitudinal and lateral acceleration (their moments about the centre of gravity) and the
/// ground under each wheel (M-0073). Small angles; the rest position is the static equilibrium of
/// the loads the [`SuspensionModel`] distributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodySuspension {
    /// Height of the body above its rest position (m, up positive).
    pub heave_m: f32,
    /// Heave rate (m/s).
    pub heave_rate_m_s: f32,
    /// Pitch (rad, nose up positive).
    pub pitch_rad: f32,
    /// Pitch rate (rad/s).
    pub pitch_rate_rad_s: f32,
    /// Roll (rad, right side down positive).
    pub roll_rad: f32,
    /// Roll rate (rad/s).
    pub roll_rate_rad_s: f32,
    /// The wheels, axle by axle, left then right.
    pub wheels: Vec<WheelSuspension>,
    /// Sprung mass (kg).
    pub mass_kg: f32,
    /// Pitch moment of inertia about the lateral axis (kg·m²).
    pub pitch_inertia_kg_m2: f32,
    /// Roll moment of inertia about the longitudinal axis (kg·m²).
    pub roll_inertia_kg_m2: f32,
    /// Height of the centre of gravity (m).
    pub cg_height_m: f32,
    /// Vertical acceleration of the body on its springs over the last step (m/s², up
    /// positive): what the scripts' `A_Trans_Z` adds to the ground's (M-0146).
    #[serde(default)]
    pub vertical_acc_m_s2: f32,
}

impl BodySuspension {
    /// The mean [`WheelSuspension::omsi_travel_m`] of the wheels at rest (m): the travel the
    /// scripts are given for axles without a body model of their own (the trailer of an
    /// articulated bus), so that their levelling holds them where it holds the head's.
    #[must_use]
    pub fn rest_travel_m(&self) -> f32 {
        let rest = self
            .wheels
            .iter()
            .map(|w| w.omsi_travel_m() + w.compression_m);
        rest.sum::<f32>() / self.wheels.len().max(1) as f32
    }

    /// A body at rest on the wheels of `axles` (longitudinal position, track width, spring rate
    /// and damping per side), carrying the static axle loads `static_loads_n` (summing to the
    /// weight).
    #[must_use]
    pub fn new(
        axles: &[AxleConfig],
        static_loads_n: &[f32],
        mass_kg: f32,
        pitch_inertia_kg_m2: f32,
        roll_inertia_kg_m2: f32,
        cg_height_m: f32,
    ) -> Self {
        let mut wheels = Vec::with_capacity(axles.len() * 2);
        for (axle, load) in axles.iter().zip(static_loads_n) {
            for side in [-1.0f32, 1.0] {
                wheels.push(WheelSuspension {
                    x_m: axle.longitudinal_m,
                    y_m: side * axle.track_width_m * 0.5,
                    spring_n_m: axle.spring_rate_n_m.max(10_000.0),
                    damping_n_s_m: axle.damping_n_s_m.max(1_000.0),
                    spring_factor: 1.0,
                    static_load_n: load * 0.5,
                    compression_m: 0.0,
                    compression_rate_m_s: 0.0,
                    load_n: load * 0.5,
                    payload_n: 0.0,
                });
            }
        }
        Self {
            heave_m: 0.0,
            heave_rate_m_s: 0.0,
            pitch_rad: 0.0,
            pitch_rate_rad_s: 0.0,
            roll_rad: 0.0,
            roll_rate_rad_s: 0.0,
            wheels,
            mass_kg: mass_kg.max(100.0),
            pitch_inertia_kg_m2: pitch_inertia_kg_m2.max(100.0),
            roll_inertia_kg_m2: roll_inertia_kg_m2.max(100.0),
            cg_height_m: cg_height_m.max(0.1),
            vertical_acc_m_s2: 0.0,
        }
    }

    /// Sets the weight of the people on board (N per axle, left and right wheel half each).
    pub fn set_payload(&mut self, axle_loads_n: &[f32]) {
        for (i, w) in self.wheels.iter_mut().enumerate() {
            w.payload_n = axle_loads_n.get(i / 2).copied().unwrap_or(0.0) * 0.5;
        }
    }

    /// The weight of the people on board (N).
    #[must_use]
    pub fn payload_n(&self) -> f32 {
        self.wheels.iter().map(|w| w.payload_n).sum()
    }

    /// Sum of the loads on the wheels of axle `index` (N).
    #[must_use]
    pub fn axle_load_n(&self, index: usize) -> f32 {
        self.wheels
            .get(index * 2..index * 2 + 2)
            .map_or(0.0, |w| w.iter().map(|w| w.load_n).sum())
    }

    /// Sum of all wheel loads (N).
    #[must_use]
    pub fn total_load_n(&self) -> f32 {
        self.wheels.iter().map(|w| w.load_n).sum()
    }
}
