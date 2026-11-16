//! One object's tick: whether it is due, and what one tick does to its state and effects.

use super::*;

/// When an object is due and with which step.
pub(super) fn due(
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
pub(super) fn tick_one(
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
