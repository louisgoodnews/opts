//! Ticking scenery objects: distance bands, a per-frame budget, and the deterministic step the
//! benchmark uses ([`advance_scenery`]).

use std::time::Instant;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use super::clock::SceneryClock;
use super::effects::{EffectTargets, SceneryObjectRuntime};
use super::scripts::{HostValues, SceneryScripts, SceneryVariableOverrides};

mod advance;
mod objects;
mod one;

use objects::tick_objects;

pub use advance::advance_scenery;

/// Longest step handed to a script after it was skipped (distance bands, budget).
pub const MAX_STEP_S: f32 = 2.0;

/// Scheduling of scenery ticks.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct SceneryRuntimeSettings {
    /// Tick in `Update` from Bevy's clock. `false`: only [`advance_scenery`] ticks.
    pub automatic: bool,
    /// Objects within this distance (m) tick every frame.
    pub near_m: f32,
    /// Objects within this distance (m) tick every `mid_interval_s`.
    pub mid_m: f32,
    /// Tick interval of the mid band (s).
    pub mid_interval_s: f32,
    /// Tick interval beyond `mid_m` (s).
    pub far_interval_s: f32,
    /// Most objects ticked in one frame; the rest wait (round robin).
    pub max_ticks_per_frame: usize,
}

impl Default for SceneryRuntimeSettings {
    fn default() -> Self {
        Self {
            automatic: true,
            near_m: 250.0,
            mid_m: 1_000.0,
            mid_interval_s: 0.25,
            far_interval_s: MAX_STEP_S,
            max_ticks_per_frame: 1_000,
        }
    }
}

/// Counters of the last frame (diagnostic overlay, profiling).
#[derive(Resource, Debug, Clone, Default)]
pub struct SceneryRuntimeStats {
    /// Objects with runtime state.
    pub objects: usize,
    /// Of those, objects with a script.
    pub scripted: usize,
    /// Objects ticked in the last frame.
    pub ticks: usize,
    /// Of those, script runs.
    pub script_ticks: usize,
    /// Time spent ticking in the last frame (µs), including the effect updates.
    pub tick_time_us: f32,
    /// Run-time diagnostics raised so far (all objects).
    pub runtime_diagnostics: usize,
    /// Round-robin start for the next frame.
    cursor: usize,
}

type ObjectQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut SceneryObjectRuntime,
        Option<&'static mut SceneryScripts>,
        &'static GlobalTransform,
    ),
>;

struct TickContext<'a> {
    dt: f32,
    force: bool,
    paused: bool,
    camera: Option<Vec3>,
    settings: &'a SceneryRuntimeSettings,
    host: &'a HostValues,
    overrides: &'a SceneryVariableOverrides,
}

/// System parameters for [`scenery_script_system`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct SceneryScriptParams<'w, 's> {
    pub time: Res<'w, Time>,
    pub settings: Res<'w, SceneryRuntimeSettings>,
    pub clock: Res<'w, SceneryClock>,
    pub overrides: Res<'w, SceneryVariableOverrides>,
    pub stats: ResMut<'w, SceneryRuntimeStats>,
    pub cameras:
        Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<crate::MainCamera>>,
    pub objects: ObjectQuery<'w, 's>,
    pub targets: EffectTargets<'w, 's>,
}

/// `Update` system: ticks scenery objects by distance from the active 3D camera.
pub fn scenery_script_system(mut p: SceneryScriptParams) {
    if !p.settings.automatic {
        return;
    }
    // A paused clock (dt = 0) still evaluates objects whose host values changed (night switch):
    // `tick_objects` then advances no script time.
    let dt = p.time.delta_secs();
    let paused = dt <= 0.0;
    let camera = p
        .cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation());
    let host = HostValues::of(&p.clock);
    let ctx = TickContext {
        dt,
        force: false,
        paused,
        camera,
        settings: &p.settings,
        host: &host,
        overrides: &p.overrides,
    };
    tick_objects(&ctx, &mut p.stats, &mut p.objects, &mut p.targets);
}
