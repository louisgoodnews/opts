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

/// State of one light at a captured step.
#[derive(Debug, Clone, PartialEq)]
pub struct LightProbe {
    /// Brightness the runtime computed (0..1).
    pub level: f32,
    /// The runtime shows the sprite (not hidden by distance or cone).
    pub shown: bool,
    /// World position (Bevy axes).
    pub pos: Vec3,
    /// Sprite diameter at full brightness (m).
    pub size_m: f32,
}

/// What the runtime holds for the object at a captured step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    /// All script variables (lower-case names).
    pub vars: BTreeMap<String, f32>,
    /// Per `[mesh]`: the animation values the mesh currently shows (empty without `[newanim]`).
    pub shown: Vec<Vec<f32>>,
    /// Lights in `.sco` order.
    pub lights: Vec<LightProbe>,
    /// Applied brightness of each light map (NaN before the first update).
    pub light_maps: Vec<f32>,
}

/// One captured step.
#[derive(Debug, Clone)]
pub struct State {
    /// `s0`, `s1`, `s2`, `s3`, `s4_<phase>`, `s0b`.
    pub name: String,
    /// Both views over black and white.
    pub pixels: Vec<ViewPixels>,
    /// The runtime's state.
    pub probe: Probe,
}

/// One script diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDiag {
    /// Severity is `Error`.
    pub error: bool,
    /// Raised while running (not while loading).
    pub runtime: bool,
    /// Script file as given.
    pub file: String,
    /// Message.
    pub message: String,
}

/// Everything the scenarios produced.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Captured states in scenario order.
    pub states: Vec<State>,
    /// Distinct diagnostics.
    pub diagnostics: Vec<ScriptDiag>,
    /// The spawned object carried a script engine.
    pub engine_found: bool,
    /// Load-time diagnostics of the engine (those after this index were raised while running).
    pub load_len: Option<usize>,
    /// The runtime entities of the current placement (taken before a seeded fault detaches them).
    handles: Handles,
}

/// Entities of the object's runtime, in `.sco` order.
#[derive(Debug, Clone, Default)]
struct Handles {
    meshes: Vec<Entity>,
    lights: Vec<Entity>,
    light_maps: Vec<Entity>,
}

/// Inputs of a run.
pub struct Ctx<'a> {
    /// The object.
    pub entry: &'a ObjectEntry,
    /// Its preflight.
    pub pre: &'a Preflight,
    /// String set A of the placement.
    pub strings: &'a [String],
    /// Where the tile places it.
    pub tile_place: mapgen::Placement,
    /// The views (framed on the expected geometry).
    pub views: &'a [View],
    /// Time budget end.
    pub deadline: Instant,
    /// Thresholds.
    pub thresholds: &'a ScriptThresholds,
    /// Seeded fault: detach the object's effects from the runtime.
    pub effects_off: bool,
}

/// Runs the scenarios; the harness is left in the static-view configuration.
///
/// # Errors
/// The object cannot be placed or a capture fails; the flag of a timed-out placement is an error
/// too.
pub fn run(h: &mut Harness, ctx: &Ctx<'_>) -> Result<Outcome> {
    h.world_mut().insert_resource(SceneryLodMode::Finest);
    let mut out = Outcome::default();
    let r = scenarios(h, ctx, &mut out);
    let world = h.world_mut();
    world.insert_resource(SceneryLodMode::All);
    world.insert_resource(SceneryVariableOverrides::default());
    world.insert_resource(SceneryClock::at(2026, 6, 29, 12.0 * 3600.0));
    r.map(|()| out)
}

fn scenarios(h: &mut Harness, ctx: &Ctx<'_>, out: &mut Outcome) -> Result<()> {
    let noon = (2026, 6, 29, 12.0 * 3600.0);
    fresh(h, ctx, noon, out)?;
    step(h, SCENARIO_STEPS, ctx);
    snap(h, ctx, "s0", out)?;
    step(h, S1_EXTRA_STEPS, ctx);
    snap(h, ctx, "s1", out)?;
    fresh(h, ctx, (2026, 6, 29, 23.0 * 3600.0), out)?;
    step(h, SCENARIO_STEPS, ctx);
    snap(h, ctx, "s2", out)?;
    fresh(h, ctx, (2026, 1, 15, 12.0 * 3600.0), out)?;
    step(h, SCENARIO_STEPS, ctx);
    snap(h, ctx, "s3", out)?;
    if has_traffic_light(ctx.pre) {
        fresh(h, ctx, noon, out)?;
        for phase in TRAFFIC_STATES {
            let mut o = SceneryVariableOverrides::default();
            o.0.insert("TrafficLightPhase".into(), phase);
            h.world_mut().insert_resource(o);
            step(h, SCENARIO_STEPS, ctx);
            snap(h, ctx, &format!("s4_{phase}"), out)?;
        }
    }
    let inputs: Vec<&str> = INPUT_VARIABLES
        .iter()
        .copied()
        .filter(|n| {
            out.states
                .first()
                .is_some_and(|s| s.probe.vars.contains_key(*n))
        })
        .collect();
    if !inputs.is_empty() {
        fresh(h, ctx, noon, out)?;
        for name in inputs {
            for value in 0..3 {
                let mut o = SceneryVariableOverrides::default();
                o.0.insert(name.into(), value as f32);
                h.world_mut().insert_resource(o);
                step(h, SCENARIO_STEPS, ctx);
                snap(h, ctx, &format!("s6_{name}_{value}"), out)?;
            }
        }
    }
    fresh(h, ctx, noon, out)?;
    step(h, SCENARIO_STEPS, ctx);
    snap(h, ctx, "s0b", out)
}

/// Places the object anew (a fresh engine with `{init}` pending), waits for the tile to settle
/// and sets the clock.
fn fresh(
    h: &mut Harness,
    ctx: &Ctx<'_>,
    clock: (i32, u32, u32, f64),
    out: &mut Outcome,
) -> Result<()> {
    h.place(Some((&ctx.entry.sco, ctx.strings, ctx.tile_place)))?;
    if !h.settle(ctx.deadline)? {
        return Err(anyhow!("the object did not settle for a script scenario"));
    }
    let world = h.world_mut();
    world.insert_resource(SceneryVariableOverrides::default());
    world.insert_resource(SceneryClock::at(clock.0, clock.1, clock.2, clock.3));
    if let Some(root) = find_root(world) {
        out.engine_found = true;
        if out.load_len.is_none() {
            out.load_len = world
                .get::<SceneryScripts>(root)
                .map(|s| s.diagnostics().len());
        }
        if let Some(rt) = world.get::<SceneryObjectRuntime>(root) {
            out.handles = Handles {
                meshes: rt.meshes.clone(),
                lights: rt.lights.clone(),
                light_maps: rt.light_maps.clone(),
            };
        }
        if ctx.effects_off {
            if let Some(mut rt) = world.get_mut::<SceneryObjectRuntime>(root) {
                rt.meshes.clear();
                rt.lights.clear();
                rt.light_maps.clear();
            }
        }
    }
    Ok(())
}

fn step(h: &mut Harness, n: usize, ctx: &Ctx<'_>) {
    let dt = ctx.thresholds.dt_s;
    let world = h.world_mut();
    for _ in 0..n {
        advance_scenery(world, dt);
    }
}

/// Captures the views and the probe of the current step and collects new diagnostics.
fn snap(h: &mut Harness, ctx: &Ctx<'_>, name: &str, out: &mut Outcome) -> Result<()> {
    h.warm_up(ctx.views);
    let pixels = h.capture_views(ctx.views)?;
    let Some(sco) = ctx.pre.object.as_ref() else {
        return Err(anyhow!("no parsed object"));
    };
    let world = h.world_mut();
    let probe = find_root(world)
        .map(|root| probe(world, root, sco, &out.handles))
        .unwrap_or_default();
    collect_diagnostics(world, out);
    out.states.push(State {
        name: name.to_string(),
        pixels,
        probe,
    });
    Ok(())
}

/// The object root that carries the script engine.
fn find_root(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, With<SceneryScripts>>()
        .iter(world)
        .next()
}

fn collect_diagnostics(world: &mut World, out: &mut Outcome) {
    let Some(root) = find_root(world) else {
        return;
    };
    let load_len = out.load_len;
    let Some(scripts) = world.get::<SceneryScripts>(root) else {
        return;
    };
    for (i, d) in scripts.diagnostics().iter().enumerate() {
        let diag = ScriptDiag {
            error: d.severity == Severity::Error,
            runtime: load_len.is_some_and(|n| i >= n),
            file: d.file.clone(),
            message: d.message.clone(),
        };
        if !out.diagnostics.contains(&diag) {
            out.diagnostics.push(diag);
        }
    }
}

/// Reads the runtime's state of the object at `root`.
fn probe(world: &World, root: Entity, sco: &SceneryObject, handles: &Handles) -> Probe {
    let mut p = Probe {
        shown: vec![Vec::new(); sco.meshes.len()],
        ..Probe::default()
    };
    if let Some(scripts) = world.get::<SceneryScripts>(root) {
        p.vars = scripts
            .with_engine(|e| e.variables())
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
    }
    for &e in &handles.meshes {
        let index = world
            .get::<Name>(e)
            .and_then(|n| n.as_str().strip_prefix("Mesh_"))
            .and_then(|s| s.parse::<usize>().ok());
        if let (Some(i), Some(rule)) = (index, world.get::<SceneryMeshRule>(e)) {
            if let Some(slot) = p.shown.get_mut(i) {
                slot.clone_from(&rule.anim.shown);
            }
        }
    }
    for &e in &handles.lights {
        if let (Some(l), Some(g), Some(v)) = (
            world.get::<SceneryLight>(e),
            world.get::<GlobalTransform>(e),
            world.get::<Visibility>(e),
        ) {
            p.lights.push(LightProbe {
                level: l.level,
                shown: *v != Visibility::Hidden,
                pos: g.translation(),
                size_m: l.size_m,
            });
        }
    }
    for &e in &handles.light_maps {
        if let Some(m) = world.get::<SceneryLightMap>(e) {
            p.light_maps.push(m.applied);
        }
    }
    p
}
