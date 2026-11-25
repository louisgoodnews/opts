//! Spawning and despawning traffic.

mod agent;
mod models;
mod params;
use super::variants::pick_slot;
use super::*;
use agent::place_and_insert_agent;
use models::build_unit_models;
use params::{SpawnContext, SpawnLocals, SpawnStores};

mod density;
mod place;
mod state;

use density::{density_plan, DensityPlan};
use place::{finish_spawn, pick_place};
use state::{log_state, mark_unusable};

/// Spawns vehicles on lanes around the camera up to the density-derived target.
pub fn spawn_traffic(
    time: Res<Time>,
    locals: SpawnLocals,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    ctx: SpawnContext,
    stores: SpawnStores,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) {
    let SpawnLocals {
        mut elapsed,
        mut since_log,
        mut rejected,
    } = locals;
    let SpawnContext {
        mut state,
        mut pool,
        settings,
        mut sched,
        env,
        chrono,
    } = ctx;
    let SpawnStores {
        mut meshes,
        mut materials,
        mut images,
    } = stores;
    *elapsed += time.delta_secs();
    if *elapsed < SPAWN_INTERVAL_S || settings.baseline == 0 {
        return;
    }
    *elapsed = 0.0;
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    let centre = DVec2::new(
        f64::from(cam.translation().x),
        -f64::from(cam.translation().z),
    );
    let plan = density_plan(&mut pool, &state, settings.baseline, centre, env.as_deref());
    log_state(&mut since_log, &plan, &state, &pool, &rejected);
    // A scheduled vehicle asked for by `plan_scheduled` goes first and does not count against the
    // target of the unscheduled traffic.
    let request = sched.request.take();
    let unscheduled = state
        .world
        .agents()
        .len()
        .saturating_sub(sched.agent_count());
    if request.is_none() && unscheduled >= plan.target {
        return;
    }
    let mut rng = pool.rng.take().unwrap_or_else(|| Rng::new(1));
    // Why an attempt found no place, counted for the diagnostic line: no vehicle ready (or no
    // group below its target), no lane in the ring around the camera, lane gone, -, in the
    // camera's view.
    let requested = request.as_ref().and_then(|r| {
        let at = state.world.graph().lane(r.lane)?.geometry.point_at(r.s);
        Some((r.lane, r.s, r.slot, at))
    });
    let result = match requested {
        Some(found) => Ok(found),
        None => pick_place(&pool, &state, (&plan.deficits, centre, cam), &mut rng),
    };
    let (lane_id, s, slot, at) = match result {
        Ok(found) => found,
        Err(reason) => {
            rejected[reason] += 1;
            pool.rng = Some(rng);
            return;
        }
    };

    finish_spawn(
        (&mut commands, &mut job),
        (&mut state, &mut pool, &mut sched),
        (&mut meshes, &mut materials, &mut images),
        (chrono.as_deref(), request),
        ((lane_id, s, slot, at), rng),
    );
}
