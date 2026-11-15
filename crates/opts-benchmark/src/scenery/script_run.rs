//! The scenario engine of the script-effect check: places the object afresh for each scenario,
//! sets the scenery clock and variable overrides, steps the runtime with `advance_scenery` and
//! captures the views and a probe of the object's runtime state at each step of interest
//! (`docs/src/architecture/scenery-benchmark.md`, Script-effect check).
//!
//! Nothing here judges pixels; [`super::script_judge`] does.

use std::collections::BTreeMap;
use std::time::Instant;

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use opts_bevy::scenery_runtime::{SceneryLight, SceneryLightMap};
use opts_bevy::{
    advance_scenery, SceneryClock, SceneryLodMode, SceneryMeshRule, SceneryObjectRuntime,
    SceneryScripts, SceneryVariableOverrides,
};
use opts_core::formats::scenery::SceneryObject;
use opts_sim::script::Severity;

use super::capture::ViewPixels;
use super::framing::View;
use super::mapgen;
use super::preflight::Preflight;
use super::script_check::{has_traffic_light, ScriptThresholds};
use super::worker::Harness;
use super::ObjectEntry;

mod probing;
mod scenarios;
mod steps;
mod types;

pub use scenarios::run;
pub use types::{Ctx, LightProbe, Outcome, Probe, ScriptDiag, State};

/// Frames of a 2 s scenario at `dt = 1/60`.
pub const SCENARIO_STEPS: usize = 120;
/// `TrafficLightPhase` states of S4: red, red and yellow, green, yellow, off (the ranges of
/// `sco.md`, one value of each).
pub const TRAFFIC_STATES: [f32; 5] = [0.0, 3.0, 6.0, 9.0, 12.0];
/// Scenery inputs S6 drives when the script (or its variable list) mentions them (lower case).
pub const INPUT_VARIABLES: [&str; 5] = [
    "switch",
    "signal",
    "nextsignal",
    "trafficlightapproach",
    "colorscheme",
];
/// Extra frames of S1 (10 s).
pub const S1_EXTRA_STEPS: usize = 600;
