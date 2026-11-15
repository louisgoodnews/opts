//! The scenario sequence: the runs of the script-effect check and the restoration of the static views.

use super::steps::{fresh, snap, step};
use super::*;

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

pub(super) fn scenarios(h: &mut Harness, ctx: &Ctx<'_>, out: &mut Outcome) -> Result<()> {
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
