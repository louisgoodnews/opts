//! Spawning and despawning traffic.

mod agent;
mod models;

use super::variants::{baseline_targets, group_lane_metres, pick_slot, time_factor};
use super::*;
use agent::place_and_insert_agent;
use models::build_unit_models;

/// What the unscheduled traffic aims for.
struct DensityPlan {
    /// Vehicles in total.
    target: usize,
    /// Per group: the target minus the vehicles alive.
    deficits: Vec<f64>,
    /// Per group: the vehicles its lane metres and time factor ask for.
    wanted: Vec<f64>,
    /// Per group: the time factor of the session's hour and day.
    factors: Vec<f64>,
    /// Per group: the vehicles alive.
    alive: Vec<usize>,
    /// The roads cannot hold the baseline.
    capped: bool,
    hour: f32,
    day_bit: u8,
}

/// The density model: per group, the lane metres within the despawn disc weighted by the group's
/// density on each lane, times its time factor for the session's hour and day, per `L0`; the
/// baseline setting spread over the groups and limited to what the roads hold.
fn density_plan(
    pool: &mut TrafficPool,
    state: &TrafficState,
    baseline: usize,
    centre: DVec2,
    env: Option<&opts_bevy::SessionEnvironment>,
) -> DensityPlan {
    let (hour, day_bit) = env.map_or((12.0, day::WEEKDAY), |e| (e.hour(), e.day_bit()));
    let factors: Vec<f64> = pool
        .groups
        .iter()
        .map(|g| time_factor(g, &pool.road_curve, day_bit, hour))
        .collect();
    let metres = group_lane_metres(
        state.world.graph(),
        centre,
        f64::from(DESPAWN_M),
        &pool.groups,
        &pool.defaults,
    );
    // Depot groups drive timetable runs only, never as unscheduled traffic.
    let metres: Vec<f64> = metres
        .iter()
        .zip(&pool.groups)
        .map(|(m, g)| if g.scheduled { 0.0 } else { *m })
        .collect();
    // The baseline setting spread over the groups, scaled by the map's factors for this hour and
    // day, limited to what the roads hold.
    let (targets, capped) = baseline_targets(&metres, &factors, baseline);
    let wanted: Vec<f64> = metres
        .iter()
        .zip(&factors)
        .map(|(m, f)| m * f / METRES_OF_LANE_PER_VEHICLE)
        .collect();
    if capped != pool.capped {
        pool.capped = capped;
        if capped {
            tracing::info!(
                baseline = baseline,
                room = ?targets.iter().sum::<f64>(),
                "the roads are full: every group scaled down in proportion"
            );
        } else {
            tracing::info!(baseline = baseline, "the roads hold the baseline again");
        }
    }
    let target = targets.iter().sum::<f64>().round() as usize;
    // Vehicles per group.
    let TrafficPool {
        agent_groups,
        groups,
        ..
    } = &mut *pool;
    agent_groups.retain(|id, _| state.world.agent(*id).is_some());
    let mut alive = vec![0usize; groups.len()];
    for &g in agent_groups.values() {
        if let Some(a) = alive.get_mut(g) {
            *a += 1;
        }
    }
    let deficits: Vec<f64> = targets
        .iter()
        .zip(&alive)
        .map(|(t, &a)| t - a as f64)
        .collect();
    DensityPlan {
        target,
        deficits,
        wanted,
        factors,
        alive,
        capped,
        hour,
        day_bit,
    }
}

/// A line every 15 s says why no vehicle appears (no road, no density, vehicles not loaded).
fn log_state(
    since_log: &mut f32,
    plan: &DensityPlan,
    state: &TrafficState,
    pool: &TrafficPool,
    rejected: &[u32; 5],
) {
    *since_log += SPAWN_INTERVAL_S;
    if *since_log >= 15.0 {
        *since_log = 0.0;
        let count = |f: fn(&SlotState) -> bool| pool.slots.iter().filter(|s| f(&s.state)).count();
        tracing::debug!(
            target = plan.target,
            agents = state.world.agents().len(),
            road_km = state.road_length_m / 1000.0,
            hour = plan.hour,
            day_bit = plan.day_bit,
            time_factors = ?plan.factors,
            wanted = ?plan.wanted,
            alive = ?plan.alive,
            capped = plan.capped,
            ready = count(|s| matches!(s, SlotState::Ready(_))),
            pending = count(|s| matches!(s, SlotState::Pending)),
            failed = count(|s| matches!(s, SlotState::Failed)),
            rejected = ?*rejected,
            "traffic state"
        );
    }
}

/// Picks a group's vehicle type and a lane for it in the ring around the camera. The error says
/// why no place was found: no vehicle ready (or no group below its target), no lane in the ring,
/// lane gone, in the camera's view.
fn pick_place(
    pool: &TrafficPool,
    state: &TrafficState,
    (deficits, centre, cam): (&[f64], DVec2, &GlobalTransform),
    rng: &mut Rng,
) -> Result<(opts_traffic::LaneId, f64, usize, DVec2), usize> {
    let fwd = cam.forward();
    let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
    // The group and vehicle type come first: bus and truck lanes admit only their own, and
    // the lane is drawn by length × the group's density on it.
    let slot = pick_slot(pool, deficits, rng).ok_or(0_usize)?;
    let class = match &pool.slots[slot].state {
        SlotState::Ready(v) => v.class(),
        _ => VehicleClass::Car,
    };
    let group = pool.groups[pool.slots[slot].group].density_group;
    let defaults = &pool.defaults;
    let (lane_id, s) = state
        .world
        .graph()
        .weighted_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, rng, |l| {
            if l.kind == LaneKind::Road && l.length() > 6.0 && l.rules.allows(class) {
                l.length() * l.rules.density.for_group(group, defaults)
            } else {
                0.0
            }
        })
        .ok_or(1_usize)?;
    let lane = state.world.graph().lane(lane_id).ok_or(2_usize)?;
    let at = lane.geometry.point_at(s);
    // Not in plain view of the camera.
    let to = (at - centre).normalize_or_zero();
    if view.dot(to) > 0.5 && at.distance(centre) < 160.0 {
        return Err(4);
    }
    Ok((lane_id, s, slot, at))
}

/// Builds the models of the vehicle picked for `slot` and puts its agent on the lane; a vehicle
/// whose meshes are not ready is tried again later (or given up).
fn finish_spawn(
    (commands, job): (&mut Commands, &mut WorldLoadJob),
    (state, pool, sched): (&mut TrafficState, &mut TrafficPool, &mut ScheduledTraffic),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (chrono, request): (Option<&ChronoState>, Option<ScheduledRequest>),
    ((lane_id, s, slot, at), mut rng): ((opts_traffic::LaneId, f64, usize, DVec2), Rng),
) {
    pool.built += 1;
    let repaint_seed = crate::new_game_screen::repaints::instance_seed(&[slot as u64, pool.built]);
    let SlotState::Ready(variant) = &pool.slots[slot].state else {
        pool.rng = Some(rng);
        if let Some(r) = &request {
            sched.failed(r);
        }
        return;
    };
    let fleet = &pool.slots[slot].fleet;
    let models = match build_unit_models(
        commands,
        job,
        meshes,
        materials,
        images,
        variant,
        at,
        fleet,
        repaint_seed,
        chrono.as_deref(),
        request.is_some(),
    ) {
        Ok(m) => m,
        Err(unusable) => {
            let name = variant.name.clone();
            tracing::debug!(vehicle = %name, "AI vehicle meshes not ready");
            if let Some(r) = &request {
                sched.failed(r);
            }
            mark_unusable(pool, slot, unusable);
            pool.rng = Some(rng);
            return;
        }
    };
    place_and_insert_agent(
        commands,
        job,
        state,
        pool,
        sched,
        slot,
        lane_id,
        s,
        at,
        request.as_ref(),
        models,
        &mut rng,
    );
    pool.rng = Some(rng);
}

/// A vehicle whose meshes cannot be loaded now: a slot that can never load is failed, any other
/// gets a few more tries.
fn mark_unusable(pool: &mut TrafficPool, slot: usize, unusable: bool) {
    if unusable {
        pool.slots[slot].state = SlotState::Failed;
    } else {
        pool.slots[slot].fails += 1;
        if pool.slots[slot].fails >= MAX_LOAD_TRIES {
            pool.slots[slot].state = SlotState::Failed;
        }
    }
}

/// Spawns vehicles on lanes around the camera up to the density-derived target.
#[allow(clippy::too_many_arguments)]
pub fn spawn_traffic(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut since_log: Local<f32>,
    mut rejected: Local<[u32; 5]>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    mut state: ResMut<TrafficState>,
    mut pool: ResMut<TrafficPool>,
    settings: Res<TrafficSettings>,
    mut sched: ResMut<ScheduledTraffic>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    chrono: Option<Res<crate::in_game::omsi_world::ChronoState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) {
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
