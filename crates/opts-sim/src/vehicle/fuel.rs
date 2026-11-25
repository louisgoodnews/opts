//! Fuel (diesel) and battery (electric) charge: depletion with the throttle, a torque cut at
//! empty and the low-level warning (M-0123). Engine-agnostic: no Bevy types here.
//!
//! Model constants are OPTS model assumptions, not values read from OMSI files: a diesel city bus
//! burns [`DIESEL_IDLE_L_PER_H`] at idle and [`DIESEL_FULL_L_PER_H`] at full throttle; an electric
//! vehicle draws [`ELECTRIC_IDLE_UNITS_PER_H`] / [`ELECTRIC_FULL_UNITS_PER_H`] from a store of its
//! capacity in the same units. [`FuelMode::Accelerated`] multiplies the rate by
//! [`ACCELERATED_FACTOR`].

use serde::{Deserialize, Serialize};

/// Diesel burn at idle (litres per hour).
pub const DIESEL_IDLE_L_PER_H: f32 = 3.0;
/// Diesel burn at full throttle (litres per hour).
pub const DIESEL_FULL_L_PER_H: f32 = 60.0;
/// Electric draw at idle (capacity units per hour).
pub const ELECTRIC_IDLE_UNITS_PER_H: f32 = 2.0;
/// Electric draw at full throttle (capacity units per hour).
pub const ELECTRIC_FULL_UNITS_PER_H: f32 = 120.0;
/// Tank or store used when the vehicle data names no capacity (litres, or capacity units).
pub const DEFAULT_CAPACITY: f32 = 250.0;
/// Consumption multiplier of [`FuelMode::Accelerated`].
pub const ACCELERATED_FACTOR: f32 = 10.0;
/// Level (share of capacity, 0 to 1) at or below which the low-level warning shows.
pub const LOW_LEVEL: f32 = 0.10;
/// Share of the drive torque an electric vehicle keeps at an empty store (limp mode).
pub const EMPTY_ELECTRIC_TORQUE_FRACTION: f32 = 0.2;

/// Whether the vehicle burns diesel or draws from a battery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FuelKind {
    /// A combustion engine with a fuel tank.
    #[default]
    Diesel,
    /// A traction battery.
    Electric,
}

/// The player's choice of how much consumption applies (setting `Off`, `Realistic`,
/// `Accelerated`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FuelMode {
    /// No consumption: the level never changes and the torque is never cut.
    Off,
    /// The model's rates, multiplier 1.
    #[default]
    Realistic,
    /// The model's rates times [`ACCELERATED_FACTOR`].
    Accelerated,
}

impl FuelMode {
    /// The consumption multiplier of this mode (0 for `Off`).
    #[must_use]
    pub fn multiplier(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Realistic => 1.0,
            Self::Accelerated => ACCELERATED_FACTOR,
        }
    }

    /// The settings value of this mode (`off`, `realistic` or `accelerated`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Realistic => "realistic",
            Self::Accelerated => "accelerated",
        }
    }

    /// Reads a settings value; anything unknown is [`FuelMode::Realistic`].
    #[must_use]
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "off" => Self::Off,
            "accelerated" => Self::Accelerated,
            _ => Self::Realistic,
        }
    }
}

/// The charge of one vehicle: how much is left in its tank or battery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuelState {
    /// Diesel or electric.
    pub kind: FuelKind,
    /// Full content (litres, or capacity units for an electric store); positive.
    pub capacity: f32,
    /// Content now, between 0 and `capacity`.
    pub content: f32,
    /// The consumption choice of the player.
    pub mode: FuelMode,
    /// Whether the vehicle's scripts own the charge (ISS-042): the host mirrors their state of
    /// charge and the model does not drain it again. A file without the field reads `false`.
    #[serde(default)]
    pub script_owned: bool,
}

impl FuelState {
    /// A full store of `kind` with `capacity` (values below 1 are raised to 1).
    #[must_use]
    pub fn full(kind: FuelKind, capacity: f32) -> Self {
        let capacity = capacity.max(1.0);
        Self {
            kind,
            capacity,
            content: capacity,
            mode: FuelMode::default(),
            script_owned: false,
        }
    }

    /// The level as a share of the capacity, 0 to 1.
    #[must_use]
    pub fn level(&self) -> f32 {
        (self.content / self.capacity).clamp(0.0, 1.0)
    }

    /// Sets the level to `share` of the capacity (clamped to 0 to 1).
    pub fn set_level(&mut self, share: f32) {
        self.content = self.capacity * share.clamp(0.0, 1.0);
    }

    /// Fills the store to the top (the refuel action, or a recharge at the depot).
    pub fn refill(&mut self) {
        self.content = self.capacity;
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content <= 0.0
    }

    /// Whether the low-level warning shows (level at or below [`LOW_LEVEL`]).
    #[must_use]
    pub fn is_low(&self) -> bool {
        self.level() <= LOW_LEVEL
    }

    /// Consumption rate in content units per second at the given throttle (0 to 1).
    #[must_use]
    pub fn rate_per_s(&self, throttle: f32) -> f32 {
        let (idle, full) = match self.kind {
            FuelKind::Diesel => (DIESEL_IDLE_L_PER_H, DIESEL_FULL_L_PER_H),
            FuelKind::Electric => (ELECTRIC_IDLE_UNITS_PER_H, ELECTRIC_FULL_UNITS_PER_H),
        };
        let load = throttle.clamp(0.0, 1.0);
        (idle + (full - idle) * load) / 3600.0 * self.mode.multiplier()
    }

    /// Depletes the store for `dt` seconds at the given throttle. `Off` changes nothing, and so
    /// does a charge the scripts own (they drain it themselves, see [`Self::follow_charge`]).
    pub fn step(&mut self, throttle: f32, dt: f32) {
        if self.mode == FuelMode::Off || self.script_owned || dt <= 0.0 {
            return;
        }
        self.content = (self.content - self.rate_per_s(throttle) * dt).max(0.0);
    }

    /// The share of the drive torque the vehicle keeps: 1 while there is content (or the mode
    /// is `Off`), 0 for an empty diesel tank, [`EMPTY_ELECTRIC_TORQUE_FRACTION`] for an empty
    /// battery.
    #[must_use]
    pub fn torque_factor(&self) -> f32 {
        if self.mode == FuelMode::Off || !self.is_empty() {
            return 1.0;
        }
        match self.kind {
            FuelKind::Diesel => 0.0,
            FuelKind::Electric => EMPTY_ELECTRIC_TORQUE_FRACTION,
        }
    }
}

mod charge;
mod sim;

pub use charge::{ELECTRIC_CAPACITY_CONST, ELECTRIC_CHARGE_VAR};

#[cfg(test)]
mod stopped_tests;
#[cfg(test)]
mod tests;
