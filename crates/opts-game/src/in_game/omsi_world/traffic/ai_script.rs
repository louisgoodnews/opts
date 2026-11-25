//! The scripts of scheduled AI vehicles (M-0112, the one-script path of M-0103).
//!
//! An unscheduled AI vehicle is a table of animation variables ([`opts_bevy::AiVariables`]). A
//! scheduled bus needs more: its doors open at a stop and its destination display shows the
//! line, and both are done by the vehicle's own scripts (`main_AI.osc`, the door scripts,
//! `VMatrix.osc`). So a scheduled vehicle gets its script engine, run on `{frame_ai}` with the
//! variables OMSI's host gives an AI vehicle (`AI`, `AI_Engine`, `AI_Scheduled_AtStation`, the
//! `PAX_*_Req` flags of passengers who want in or out, `AI_Target_Index` through the trigger
//! `ai_scheduled_settarget`), and the timetable callbacks answer from the run's schedule. The
//! traffic engine still drives the vehicle; the scripts only show it.

use std::path::Path;
use std::sync::{Arc, RwLock};

use opts_bevy::route::{keep_target, RouteHandle, VehicleRoute, TARGET_OFFER_S};
use opts_bevy::VehicleScripts;
use opts_core::formats::script_texture::CpuScriptTexture;
use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::inputs::{feed, HostInputs, WEARLIFESPAN_NO_BREAKDOWNS};

use super::run_schedule::RunRef;
use super::*;

mod engine;
mod fallback;
pub(in crate::in_game::omsi_world) use engine::engine_for;
pub(super) use fallback::{is_fallback, ScriptDrive};
mod frame;
pub use frame::step_ai_scripts;

/// Scripts run within this distance (m) of the camera; farther vehicles keep their last state.
pub(in crate::in_game::omsi_world) const SCRIPT_RADIUS_M: f64 = 150.0;
/// Seconds between two runs of the scripts' frame.
pub(in crate::in_game::omsi_world) const FRAME_S: f32 = 0.1;
/// Share of the dwell (from the arrival) in which passengers want to get in and out, so the doors
/// are open for that long and closed before the vehicle leaves.
pub(in crate::in_game::omsi_world) const DOORS_OPEN_SHARE: f64 = 0.5;
/// Passenger doors the request flags are set for.
const DOORS: usize = opts_sim::script::inputs::DOORS;

/// Marks a scheduled AI vehicle whose scripts run, and keeps the time since their last frame.
#[derive(Component, Default)]
pub struct AiScript {
    since: f32,
    doors: Doors,
}

/// Where the doors are in the stop: the host sets `AI_Scheduled_AtStation` to `1` on arrival (the
/// door scripts open), to `-1` when the passengers are through (they close) and the scripts set
/// it back to `0` once the doors are shut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::in_game::omsi_world) enum Doors {
    #[default]
    Away,
    Open,
    Closing,
}

/// The doors' next phase and the value to write to `AI_Scheduled_AtStation` (`None`: leave it, the
/// scripts own it once they have it): `at_stop` is whether the vehicle stands in a stop, `open`
/// whether the passengers still want in and out.
pub(in crate::in_game::omsi_world) fn next_doors(
    doors: Doors,
    at_stop: bool,
    open: bool,
) -> (Doors, Option<f32>) {
    match (at_stop, doors) {
        (false, Doors::Away) => (Doors::Away, None),
        (false, _) => (Doors::Away, Some(0.0)),
        (true, Doors::Away) => (Doors::Open, Some(1.0)),
        (true, Doors::Open) if !open => (Doors::Closing, Some(-1.0)),
        (true, d) => (d, None),
    }
}

/// Loads the head unit's scripts into an engine, with the depot and the timetable of `run` as
/// callbacks, and puts them on the vehicle `root`. `canvases` are the script textures the model's
/// materials show. Returns whether the vehicle has scripts.
pub(super) fn attach(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    bus_path: &Path,
    (canvases, run): (Arc<RwLock<Vec<CpuScriptTexture>>>, Option<&RunRef>),
    (seed, setvars): (u64, &[(String, String)]),
) -> bool {
    let Some((scripts, vehicle_route)) = engine_for(job, bus_path, canvases, run, seed, setvars)
    else {
        return false;
    };
    commands
        .entity(root)
        .insert((scripts, vehicle_route, AiScript::default()));
    true
}

#[cfg(test)]
mod tests;
