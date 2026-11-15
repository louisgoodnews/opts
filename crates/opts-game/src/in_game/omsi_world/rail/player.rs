//! The player's rail vehicle: the [`RailPlayer`] component, its construction and consist dynamics.

use bevy::prelude::*;
use opts_core::formats::vehicles::BusManifest;
use opts_sim::rail::{RailDynamics, Train};

use super::{DEFAULT_WHEEL_RADIUS_M, SNAP_MAX_M};

/// A player vehicle that runs on rails (`[type] 2`): the chassis is held on the track and its
/// speed comes from [`RailDynamics`] instead of the road drivetrain.
#[derive(Component)]
pub struct RailPlayer {
    pub(super) train: Option<Train>,
    pub(super) speed: f32,
    pub(super) dynamics: RailDynamics,
    /// Half bogie span of the head: the train point is its front bogie, its origin lies this far
    /// behind it.
    pub(super) span: f32,
    pub(super) gave_up: bool,
    /// How far from its spawn point a track is looked for (m).
    pub(super) snap_m: f32,
    /// The vehicle backs up: its head moves back along the track it ran over.
    pub(super) backing: bool,
    /// Radius of the driven wheels (m): the scripts' torque pulls the train with `M_Wheel / r`.
    pub(super) wheel_radius_m: f32,
}

impl RailPlayer {
    /// A tram of `mass_kg` waiting for its track.
    #[must_use]
    pub fn new(mass_kg: f32, span: f32) -> Self {
        Self {
            train: None,
            speed: 0.0,
            dynamics: RailDynamics::tram(mass_kg),
            span,
            gave_up: false,
            snap_m: SNAP_MAX_M,
            wheel_radius_m: DEFAULT_WHEEL_RADIUS_M,
            backing: false,
        }
    }

    /// Looks for a track up to `metres` from the spawn point (a formation: `FORMATION_SNAP_M`).
    #[must_use]
    pub fn with_snap(mut self, metres: f32) -> Self {
        self.snap_m = metres;
        self
    }
}

impl RailPlayer {
    /// Replaces the dynamics by those of the whole consist (total, powered and braked mass).
    pub fn set_consist(&mut self, load: opts_sim::rail::ConsistLoad) {
        self.dynamics = RailDynamics::for_consist(load);
    }

    /// The mass the dynamics move (kg).
    #[must_use]
    pub fn mass_kg(&self) -> f32 {
        self.dynamics.mass_kg
    }
}

/// Whether a manifest is a rail vehicle.
#[must_use]
pub fn is_rail(manifest: &BusManifest) -> bool {
    manifest.vehicle_type == Some(2)
}
