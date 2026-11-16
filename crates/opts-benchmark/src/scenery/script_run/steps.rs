//! One scenario step: placing the object afresh, advancing the runtime, and capturing a state.

use super::types::Handles;
use super::*;

use super::probing::probe;

/// Places the object anew (a fresh engine with `{init}` pending), waits for the tile to settle
/// and sets the clock.
pub(super) fn fresh(
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

pub(super) fn step(h: &mut Harness, n: usize, ctx: &Ctx<'_>) {
    let dt = ctx.thresholds.dt_s;
    let world = h.world_mut();
    for _ in 0..n {
        advance_scenery(world, dt);
    }
}

/// Captures the views and the probe of the current step and collects new diagnostics.
pub(super) fn snap(h: &mut Harness, ctx: &Ctx<'_>, name: &str, out: &mut Outcome) -> Result<()> {
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
pub(super) fn find_root(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, With<SceneryScripts>>()
        .iter(world)
        .next()
}

pub(super) fn collect_diagnostics(world: &mut World, out: &mut Outcome) {
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
