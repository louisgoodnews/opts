//! Weather of the session: the preset catalogue, the resolved [`WeatherConditions`] and the pure
//! functions that turn conditions into effect parameters (light scales, fog, sky veil, clouds,
//! precipitation, wet and snowy surfaces).
//!
//! The presets come from the install's `Weather` folder ([`opts_core::formats::weather`]); five
//! built-in presets (the ids of earlier versions) cover a missing install and old saves. The
//! rendering systems live in the game (`opts_game::in_game::weather`); everything here is
//! deterministic and tested without a window. Design: `docs/src/architecture/weather.md`.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use bevy::prelude::*;
use opts_core::formats::weather::{
    omsi_language, CloudCover, CloudLayer, CloudTypes, Fog, PrecipKind, PresetFile, WeatherFile,
    WeatherFolder,
};

mod catalogue;
mod effects;
mod fog;
pub use catalogue::{alias_of, builtin_preset, legacy_id, WeatherCatalogue, BUILTIN_IDS};
pub use effects::*;
pub use fog::{
    cloud_tint, effective_cover, fog_color, fog_density, fog_density_for, fog_layer_thickness,
    fogginess, ground_fog, ground_fog_view, light_scales, light_scales_for_cover, sky_veil,
    sky_veil_clear, FogParams, GroundFogView, LightScales, FOG_LAYER_ABOVE, FOG_LAYER_INSIDE,
    FOG_LAYER_LOOK_UP_DEG, FOG_LAYER_LOOK_UP_GAIN, FOG_LAYER_MAX_VISIBILITY_M,
    FOG_LAYER_THICKNESS_PER_VISIBILITY, FOG_LAYER_THICKNESS_RANGE_M, FOG_MAX_SATURATION,
    FOG_MAX_VISIBILITY_M,
};

mod conditions;
mod kind;
mod resolve;

pub use conditions::{absolute_humidity, CloudSpec, WeatherConditions, WeatherVars};
pub use kind::WeatherKind;

#[cfg(test)]
mod tests;
