//! The session environment: date, time of day, weather and the values derived from them (sun,
//! night, light attenuation, temperature, day of the week). The weather is a preset id, resolved
//! to [`WeatherConditions`] through the [`WeatherCatalogue`] (`weather.rs`).
//!
//! [`SessionEnvironment`] is the one source of these values for a running session. It is written
//! only through [`EnvironmentChange`] messages (session start, the pause menu's *Environment*
//! page, `sim.set_time`, restoring a save) and by its own clock, which moves with
//! `Time<Virtual>` × [`SessionEnvironment::rate`]. Every jump (a change of date, time or weather)
//! bumps [`SessionEnvironment::revision`], so consumers that cache (the sky texture) know to
//! refresh. Design: `docs/src/architecture/session-environment.md`.
//!
//! A change of the date that the map content is built for (`map_date`, Chrono) is announced with
//! [`MapDateChanged`]; the game's Chrono handler (`omsi_world::apply_map_date_changes`) reports
//! its progress in [`ChronoRebuild`], which the pause menu observes.

use std::fmt;

use bevy::prelude::*;
use opts_core::formats::map::lists::day;
use serde::{Deserialize, Serialize};

use crate::moon::{moon_state, MoonState};
pub use crate::weather::WeatherKind;
use crate::weather::{
    effective_cover, light_scales_for_cover, WeatherCatalogue, WeatherConditions,
};
use opts_core::formats::weather::PrecipKind;

mod calendar;
pub use calendar::*;

mod types;
pub use types::*;

mod reads;

mod conditions;

mod update;

mod systems;
pub use systems::*;


/// The environment systems: the clock, then the changes. A host limits them to the session with
/// `configure_sets`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvironmentSet;

/// Registers the environment resource, its messages and systems (once per app).
#[derive(Default)]
pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionEnvironment>()
            .init_resource::<WeatherCatalogue>()
            .init_resource::<crate::weather::WeatherEffects>()
            .init_resource::<ChronoRebuild>()
            .add_message::<EnvironmentChange>()
            .add_message::<MapDateChanged>()
            .add_systems(
                Update,
                (
                    tick_environment,
                    refresh_weather_on_catalogue_change,
                    apply_environment_changes,
                )
                    .chain()
                    .in_set(EnvironmentSet),
            );
    }
}

#[cfg(test)]
mod tests;
