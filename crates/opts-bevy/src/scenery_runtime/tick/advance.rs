//! The deterministic step of the scenery benchmark: every object, at a given time step.

use super::objects::tick_objects;
use super::*;

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
