//! The alert slot (UI-P13): one alert at a time, short, at the cluster, never modal.
//!
//! Sources write [`RaiseAlert`] and [`ClearAlert`]. The queue shows the most severe, newest alert;
//! *critical* alerts stay until their source clears them, the others leave after
//! [`AUTO_SECONDS`]. The slot has a fixed height, so an alert appearing moves nothing.

use bevy::prelude::*;

use super::keys;
use super::values::HudValues;

/// Seconds a non-critical alert stays.
pub const AUTO_SECONDS: f32 = 6.0;
/// Speed above which a parking brake in use is critical, km/h.
pub const BRAKE_CRITICAL_KMH: f32 = 15.0;
/// Speed above which a parking brake in use is worth a warning, km/h.
pub const BRAKE_WARN_KMH: f32 = 3.0;

mod queue;
mod systems;
mod weather;
pub use opts_game_state::alerts::{AlertSeverity, RaiseAlert};
pub use queue::{ActiveAlert, AlertQueue, ClearAlert};
pub use systems::{parking_brake_alert, update_queue};
pub use weather::weather_alerts;

#[cfg(test)]
mod tests;
