//! What a vehicle file says about its own dynamics (M-0073), and where each value came from.
//!
//! The world physics is shared (gravity, ground contact, friction, the solver); the vehicle
//! brings its mass, centre of gravity, inertia, rolling resistance and axles. Every value of the
//! spec carries its [`Source`]: read from the file, estimated from other file data, or a default
//! because the file says nothing. `docs/src/simulation/vehicle-dynamics.md` lists the fields and
//! how many files of the installation state them.

use opts_core::formats::vehicles::bus::BusManifest;
use serde::{Deserialize, Serialize};

use super::consist::GRAVITY;
use super::drivetrain::ROLLING_RESISTANCE_COEFF;

/// Where a value of the spec comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Source {
    /// Stated by the vehicle file.
    File,
    /// Derived from other data of the file (the bounding box).
    Estimate,
    /// The file says nothing usable: the generic transit-bus default.
    #[default]
    Default,
}

impl Source {
    /// A short stable name for the control protocol.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Estimate => "estimate",
            Self::Default => "default",
        }
    }
}

/// A value and its source.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sourced {
    /// The value, in the unit of the field.
    pub value: f32,
    /// Where it comes from.
    pub source: Source,
}

impl Sourced {
    fn file(value: f32) -> Self {
        Self {
            value,
            source: Source::File,
        }
    }
    fn estimate(value: f32) -> Self {
        Self {
            value,
            source: Source::Estimate,
        }
    }
    fn default(value: f32) -> Self {
        Self {
            value,
            source: Source::Default,
        }
    }
}

/// Mass assumed for a vehicle that states none (kg): a 12 m transit bus.
pub const DEFAULT_MASS_KG: f32 = 11_500.0;
/// Height of the centre of gravity assumed when the file states none (m).
pub const DEFAULT_CG_HEIGHT_M: f32 = 1.15;
/// Yaw inertia assumed when neither the file nor a bounding box gives one (kg·m²).
pub const DEFAULT_YAW_INERTIA_KG_M2: f32 = 120_000.0;
/// Smallest and largest rolling resistance coefficient taken from a file (force over weight; the
/// installed files state 0.001 to 0.02 at the 10th to 90th percentile, at most 0.1).
pub const ROLLING_COEFF_RANGE: (f32, f32) = (0.0005, 0.05);

/// The dynamics a vehicle states about itself.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleDynamicsSpec {
    /// Empty mass (`[mass]`), kg.
    pub mass_kg: Sourced,
    /// Height of the centre of gravity above the ground (`[schwerpunkt]`), m.
    pub cg_height_m: Sourced,
    /// Yaw moment of inertia (`[momentofintertia]`, else lines 2 to 4 of `[mass]`; z), kg·m².
    pub yaw_inertia_kg_m2: Sourced,
    /// Rolling resistance as force over weight (`[rollwiderstand]`, a force in newtons, divided
    /// by the stated mass and `g`).
    pub rolling_coefficient: Sourced,
}

impl Default for VehicleDynamicsSpec {
    fn default() -> Self {
        Self {
            mass_kg: Sourced::default(DEFAULT_MASS_KG),
            cg_height_m: Sourced::default(DEFAULT_CG_HEIGHT_M),
            yaw_inertia_kg_m2: Sourced::default(DEFAULT_YAW_INERTIA_KG_M2),
            rolling_coefficient: Sourced::default(ROLLING_RESISTANCE_COEFF),
        }
    }
}

impl VehicleDynamicsSpec {
    /// The spec of a vehicle file.
    #[must_use]
    pub fn from_manifest(manifest: &BusManifest) -> Self {
        let extras = &manifest.extras;
        let mass = manifest
            .mass_tonnes
            .filter(|&t| t > 0.5)
            .map(|t| t * 1_000.0);
        let mass_kg = mass.map_or(Sourced::default(DEFAULT_MASS_KG), Sourced::file);

        let cg_height_m = extras
            .centre_of_gravity_height
            .filter(|&h| h > 0.05 && h < 4.0)
            .map_or(Sourced::default(DEFAULT_CG_HEIGHT_M), Sourced::file);

        // Yaw inertia: the stated z moment (t·m²), else a box of the bounding box's width and
        // length with the vehicle's mass, else the old transit-bus figure.
        let yaw_inertia_kg_m2 = if let Some(z) = extras
            .inertia_t_m2
            .map(|i| i[2] * 1_000.0)
            .filter(|&z| z > 500.0)
        {
            Sourced::file(z)
        } else if let Some((size, _)) = manifest
            .bounding_box
            .filter(|(s, _)| s.x > 0.3 && s.y > 0.5)
        {
            Sourced::estimate(mass_kg.value * (size.x * size.x + size.y * size.y) / 12.0)
        } else {
            Sourced::default(DEFAULT_YAW_INERTIA_KG_M2)
        };

        let rolling_coefficient = match (extras.roll_resistance_n.filter(|&r| r > 0.0), mass) {
            (Some(newtons), Some(kg)) => Sourced::file(
                (newtons / (kg * GRAVITY)).clamp(ROLLING_COEFF_RANGE.0, ROLLING_COEFF_RANGE.1),
            ),
            _ => Sourced::default(ROLLING_RESISTANCE_COEFF),
        };

        Self {
            mass_kg,
            cg_height_m,
            yaw_inertia_kg_m2,
            rolling_coefficient,
        }
    }
}

#[cfg(test)]
mod tests;
