//! Ticking scenery objects: distance bands, a per-frame budget, and the deterministic step the
//! benchmark uses ([`advance_scenery`]).

use std::time::Instant;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use super::clock::SceneryClock;
use super::effects::{EffectTargets, SceneryObjectRuntime};
use super::scripts::{HostValues, SceneryScripts, SceneryVariableOverrides};

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

/// When an object is due and with which step.
fn due(
    rt: &SceneryObjectRuntime,
    scripted: bool,
    host: &HostValues,
    interval: f32,
    force: bool,
    paused: bool,
) -> bool {
    if force || !rt.updated {
        return true;
    }
    if paused {
        // The simulation clock stands still: scripts and animations must not advance, but the
        // lights still follow the host (night switch), so the Environment page previews them.
        return rt.last_night != host.night;
    }
    if scripted {
        return rt.pending_s >= interval;
    }
    // Objects without a script change only with host values.
    rt.last_night != host.night || (rt.signal.is_some() && rt.pending_s >= interval)
}

/// Ticks one object and applies its effects; returns whether a script ran.
fn tick_one(
    rt: &mut SceneryObjectRuntime,
    scripts: Option<&mut SceneryScripts>,
    host: &HostValues,
    overrides: &SceneryVariableOverrides,
    targets: &mut EffectTargets,
    stats: &mut SceneryRuntimeStats,
) -> bool {
    let dt = rt.pending_s.min(MAX_STEP_S);
    rt.pending_s = 0.0;
    rt.last_night = host.night;
    let first = !rt.updated;
    rt.updated = true;
    let dt_effects = if first { 0.0 } else { dt };
    match scripts {
        Some(s) => {
            stats.runtime_diagnostics += s.tick(host, rt.phase(host), overrides, dt);
            let engine = s.engine_mut().map(|e| &*e);
            targets.apply(rt, engine, host, dt_effects);
            true
        }
        None => {
            targets.apply(rt, None, host, dt_effects);
            false
        }
    }
}

struct TickContext<'a> {
    dt: f32,
    force: bool,
    paused: bool,
    camera: Option<Vec3>,
    settings: &'a SceneryRuntimeSettings,
    host: &'a HostValues,
    overrides: &'a SceneryVariableOverrides,
}

/// Ticks the objects in `objects`. `force` ticks every object regardless of distance and
/// budget (the benchmark's deterministic step).
fn tick_objects(
    ctx: &TickContext<'_>,
    stats: &mut SceneryRuntimeStats,
    objects: &mut ObjectQuery,
    targets: &mut EffectTargets,
) {
    let started = Instant::now();
    let total = objects.iter().len();
    let start = if total == 0 { 0 } else { stats.cursor % total };
    let (mut ticks, mut script_ticks, mut scripted, mut last) = (0, 0, 0, start);
    // Two passes from the round-robin cursor so the same objects do not always go first.
    for pass in 0..2 {
        for (i, (mut rt, scripts, transform)) in objects.iter_mut().enumerate() {
            if (pass == 0) != (i >= start) {
                continue;
            }
            rt.pending_s += ctx.dt;
            if scripts.is_some() {
                scripted += 1;
            }
            let interval = match ctx.camera {
                _ if ctx.force => 0.0,
                None => ctx.settings.mid_interval_s,
                Some(cam) => {
                    let d = transform.translation().distance(cam);
                    if d <= ctx.settings.near_m {
                        0.0
                    } else if d <= ctx.settings.mid_m {
                        ctx.settings.mid_interval_s
                    } else {
                        ctx.settings.far_interval_s
                    }
                }
            };
            if !due(
                &rt,
                scripts.is_some(),
                ctx.host,
                interval,
                ctx.force,
                ctx.paused,
            ) {
                continue;
            }
            if !ctx.force && ticks >= ctx.settings.max_ticks_per_frame {
                continue;
            }
            ticks += 1;
            last = i + 1;
            let ran = tick_one(
                &mut rt,
                scripts.map(Mut::into_inner),
                ctx.host,
                ctx.overrides,
                targets,
                stats,
            );
            script_ticks += usize::from(ran);
        }
    }
    stats.cursor = if ticks >= ctx.settings.max_ticks_per_frame {
        last
    } else {
        0
    };
    stats.objects = total;
    stats.scripted = scripted;
    stats.ticks = ticks;
    stats.script_ticks = script_ticks;
    stats.tick_time_us = started.elapsed().as_secs_f32() * 1e6;
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

/// System parameters of [`advance_scenery`].
type AdvanceParams<'w, 's> = (
    Res<'w, SceneryClock>,
    Res<'w, SceneryVariableOverrides>,
    Res<'w, SceneryRuntimeSettings>,
    ResMut<'w, SceneryRuntimeStats>,
    ObjectQuery<'w, 's>,
    EffectTargets<'w, 's>,
);

/// One deterministic step of `dt_s` seconds for **every** scenery object: moves the
/// [`SceneryClock`] on, runs `{init}` where pending and `{frame}`, and applies `[visible]`,
/// `[newanim]`, light levels and light maps. For hosts that drive time themselves (the scenery
/// benchmark); set [`SceneryRuntimeSettings::automatic`] to `false` so `Update` does not tick as
/// well.
pub fn advance_scenery(world: &mut World, dt_s: f32) {
    world.init_resource::<SceneryClock>();
    world.init_resource::<SceneryVariableOverrides>();
    world.init_resource::<SceneryRuntimeSettings>();
    world.init_resource::<SceneryRuntimeStats>();
    world.resource_mut::<SceneryClock>().advance(dt_s);
    let mut state: SystemState<AdvanceParams> = SystemState::new(world);
    let Ok((clock, overrides, settings, mut stats, mut objects, mut targets)) =
        state.get_mut(world)
    else {
        return;
    };
    let host = HostValues::of(&clock);
    let ctx = TickContext {
        dt: dt_s,
        force: true,
        paused: false,
        camera: None,
        settings: &settings,
        host: &host,
        overrides: &overrides,
    };
    tick_objects(&ctx, &mut stats, &mut objects, &mut targets);
    state.apply(world);
}
