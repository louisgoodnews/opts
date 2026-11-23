//! Per-lane rules (vehicle types, density, priority) and the height profile of a lane.

/// Traffic rules of a path (`[rule]` records of the owning spline or object).
///
/// Vehicle-type flags (the corpus has them only with value `0`; not confirmed in OMSI ❓): `bus`
/// makes the lane a lane **for buses**, `no_cars` keeps **cars** off it, and `trucks` marks a lane
/// **trucks may use**. Read as a whitelist (project lead, 2026-10-19): a lane with a type rule is
/// for the named types only (`bus` → buses only, `trucks` → trucks only). Trucks prefer `trucks`
/// lanes; the rules never apply to the player.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaneRules {
    /// `trafficdensity` rules by vehicle group (the rule's flag; `0` = none for that group;
    /// corpus values `0`, `0.001`, `0.1`, `0.5`, `1`, `3`, …).
    pub density: crate::density::LaneDensity,
    /// `priority`: raw value (corpus: `64` and `192`; the order is not settled ❓).
    pub priority: Option<u32>,
    /// A `trucks` rule exists (trucks may use the lane, and prefer it).
    pub trucks: bool,
    /// A `bus` rule exists.
    pub bus: bool,
    /// A `no_cars` rule exists.
    pub no_cars: bool,
    /// An `overtaking_prohib` rule exists.
    pub no_overtaking: bool,
}

impl LaneRules {
    /// Whether vehicles of `class` may use the lane. The vehicle-type rules are a whitelist
    /// ("only the named types are allowed here"): with a `bus` and/or `trucks` rule only those
    /// types may use the lane; `no_cars` additionally keeps cars off. Applies to AI vehicles
    /// only — the player's vehicle is never stopped by a rule.
    #[must_use]
    pub fn allows(&self, class: opts_car_following::VehicleClass) -> bool {
        use opts_car_following::VehicleClass::{Bus, Car, Truck};
        if class == Car && self.no_cars {
            return false;
        }
        if self.bus || self.trucks {
            return (class == Bus && self.bus) || (class == Truck && self.trucks);
        }
        true
    }
}

/// Height profile along a lane: start height and the gradients (rise over run) at both ends.
/// Between them the gradient changes linearly (`[spline]` records:
/// `rise = length · (g0 + g1) / 2`) or, when `rise` is given (`[spline_h]`), the height follows
/// the cubic Hermite ramp of the road mesh (`opts_core::formats::map::spline_height_offset`,
/// `docs/src/formats/map/tiles.md`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Elevation {
    /// Height at the start in metres.
    pub z0: f64,
    /// Gradient at the start (ratio).
    pub grad_start: f64,
    /// Gradient at the end (ratio).
    pub grad_end: f64,
    /// Explicit total height change of a `[spline_h]` record (m), if any.
    pub rise: Option<f64>,
}

impl Elevation {
    /// Height after `s` metres of a lane of `length` metres.
    #[must_use]
    pub fn at(&self, s: f64, length: f64) -> f64 {
        if length <= 1e-9 {
            return self.z0;
        }
        self.z0
            + opts_core::formats::map::spline_height_offset(
                self.grad_start,
                self.grad_end,
                self.rise,
                s,
                length,
            )
    }

    /// The same profile walked backwards.
    #[must_use]
    pub fn reversed(&self, length: f64) -> Self {
        Self {
            z0: self.at(length, length),
            grad_start: -self.grad_end,
            grad_end: -self.grad_start,
            rise: self.rise.map(|h| -h),
        }
    }
}
