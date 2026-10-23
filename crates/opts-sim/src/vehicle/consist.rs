//! Mass and axle data of the units of a consist (design D2, `docs/src/architecture/articulation-physics.md`).
//!
//! The longitudinal dynamics of an articulated vehicle act on **one body of mass `Σ mᵢ`**; what
//! the tyres can carry follows from the weight on the axles that drive or brake. This module holds
//! the per-unit input ([`UnitLoad`]) and the arithmetic on it; `VehicleSimulation::step` uses it.
//! Who fills a `UnitLoad` beyond `mass`/axles (passengers, inertia, spring data) is the business
//! of M-0073.

use opts_core::formats::vehicles::bus::BusManifest;
use serde::{Deserialize, Serialize};

use super::suspension::SuspensionModel;

/// Mass used for a unit whose `.bus` states none (the generic default of the head unit too).
pub const DEFAULT_UNIT_MASS_KG: f32 = 11_500.0;

/// Gravity (m/s²), as used by the rest of the vehicle model.
pub const GRAVITY: f32 = 9.81;

/// Rated load of an axle (N) when the file states none: the generic transit-bus axle.
pub const DEFAULT_RATED_AXLE_N: f32 = 115_000.0;

/// One axle of a unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UnitAxle {
    /// Position along the unit's own forward axis (m, `+y` forward).
    pub longitudinal_m: f32,
    /// The axle receives drive torque.
    pub is_driven: bool,
    /// Rated load of the axle (N, `achse_maxforce`): the brakes are sized for it.
    pub rated_n: f32,
}

/// Curb mass plus `count` people of `average_kg` each (kg): the mass a loaded vehicle moves
/// (M-0124). A negative average counts as 0.
#[must_use]
pub fn dynamic_mass_kg(curb_kg: f32, count: usize, average_kg: f32) -> f32 {
    curb_kg + count as f32 * average_kg.max(0.0)
}

/// Mass and axles of one unit of a consist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitLoad {
    /// Mass of the unit (kg).
    pub mass_kg: f32,
    /// Axles of the unit.
    pub axles: Vec<UnitAxle>,
}

impl UnitLoad {
    /// Reads the load of a unit from its parsed `.bus`: `mass` (tonnes; not stated or at most
    /// 0.5 t gives [`DEFAULT_UNIT_MASS_KG`]) and its axles. A unit that hangs `reversed` (its
    /// model turned by half a circle) has its axle positions mirrored.
    #[must_use]
    pub fn from_manifest(manifest: &BusManifest, reversed: bool) -> Self {
        Self::from_manifest_or(manifest, reversed, DEFAULT_UNIT_MASS_KG)
    }

    /// Like [`UnitLoad::from_manifest`], but a unit that states no usable mass takes
    /// `fallback_mass_kg`: the loader passes the mass of the unit in front (the installed
    /// consists have a median trailer/head ratio of 0.90, `consist_census`).
    #[must_use]
    pub fn from_manifest_or(manifest: &BusManifest, reversed: bool, fallback_mass_kg: f32) -> Self {
        let sign = if reversed { -1.0 } else { 1.0 };
        Self {
            mass_kg: manifest
                .mass_tonnes
                .filter(|&m| m > 0.5)
                .map_or(fallback_mass_kg, |m| m * 1_000.0),
            axles: manifest
                .axles
                .iter()
                .map(|a| UnitAxle {
                    longitudinal_m: a.longitudinal_m * sign,
                    is_driven: a.is_driven,
                    rated_n: a
                        .max_force
                        .filter(|&f| f > 0.0)
                        .map_or(DEFAULT_RATED_AXLE_N, |f| f * 1_000.0),
                })
                .collect(),
        }
    }

    /// A unit of `axles` axles of which `driven` are driven, nominally 1.5 m apart behind the
    /// origin (for tests and headless set-ups; real units come from
    /// [`UnitLoad::from_manifest`]).
    #[must_use]
    pub fn with_axles(mass_kg: f32, axles: usize, driven: usize) -> Self {
        Self {
            mass_kg,
            axles: (0..axles)
                .map(|i| UnitAxle {
                    longitudinal_m: -1.5 * i as f32,
                    is_driven: i < driven,
                    rated_n: DEFAULT_RATED_AXLE_N,
                })
                .collect(),
        }
    }

    /// Static normal force on every axle (N) with the centre of gravity at the middle of the
    /// axle span (the hitch load on the axles of the unit ahead is neglected).
    #[must_use]
    pub fn axle_loads_n(&self) -> Vec<f32> {
        let positions: Vec<f32> = self.axles.iter().map(|a| a.longitudinal_m).collect();
        let (lo, hi) = positions
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &y| {
                (lo.min(y), hi.max(y))
            });
        let cg = if positions.is_empty() {
            0.0
        } else {
            0.5 * (lo + hi)
        };
        SuspensionModel::new(1.15, cg).compute_static_loads(&positions, self.mass_kg)
    }
}

/// Summed normal force `(driven, braked)` in N over the given `(axle, load)` pairs. Every axle
/// is taken as braked (the vehicle files carry no brake layout).
#[must_use]
pub fn driven_and_braked_load_n(axles: &[(UnitAxle, f32)]) -> (f32, f32) {
    let braked: f32 = axles.iter().map(|(_, n)| n).sum();
    let driven: f32 = axles
        .iter()
        .filter(|(a, _)| a.is_driven)
        .map(|(_, n)| n)
        .sum();
    (driven, braked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(mass_kg: f32, axles: &[(f32, bool)]) -> UnitLoad {
        UnitLoad {
            mass_kg,
            axles: axles
                .iter()
                .map(|&(y, d)| UnitAxle {
                    longitudinal_m: y,
                    is_driven: d,
                    rated_n: DEFAULT_RATED_AXLE_N,
                })
                .collect(),
        }
    }

    #[test]
    fn axle_loads_sum_to_the_weight() {
        for u in [
            load(9_000.0, &[(-3.0, true)]),
            load(9_000.0, &[(-1.0, false), (-5.0, true)]),
            load(9_000.0, &[(2.0, false), (-1.0, true), (-3.0, false)]),
        ] {
            let sum: f32 = u.axle_loads_n().iter().sum();
            assert!((sum - 9_000.0 * GRAVITY).abs() < 1.0, "{sum}");
        }
    }

    #[test]
    fn driven_and_braked_sums_follow_the_flags() {
        let a = |d| UnitAxle {
            longitudinal_m: 0.0,
            is_driven: d,
            rated_n: DEFAULT_RATED_AXLE_N,
        };
        let (driven, braked) = driven_and_braked_load_n(&[(a(true), 100.0), (a(false), 50.0)]);
        assert_eq!((driven, braked), (100.0, 150.0));
    }
}
