//! Spawning and despawning traffic.

mod agent;
mod models;

use super::variants::{baseline_targets, group_lane_metres, pick_slot, time_factor};
use super::*;
use agent::place_and_insert_agent;
use models::build_unit_models;

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
    // Density model: per group, the lane metres within the despawn disc weighted by the group's
    // density on each lane, times its time factor for the session's hour and day, per `L0`.
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
    let (targets, capped) = baseline_targets(&metres, &factors, settings.baseline);
    let wanted: Vec<f64> = metres
        .iter()
        .zip(&factors)
        .map(|(m, f)| m * f / METRES_OF_LANE_PER_VEHICLE)
        .collect();
    if capped != pool.capped {
        pool.capped = capped;
        if capped {
            tracing::info!(
                baseline = settings.baseline,
                room = ?targets.iter().sum::<f64>(),
                "the roads are full: every group scaled down in proportion"
            );
        } else {
            tracing::info!(
                baseline = settings.baseline,
                "the roads hold the baseline again"
            );
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
    // A line every 15 s says why no vehicle appears (no road, no density, vehicles not loaded).
    *since_log += SPAWN_INTERVAL_S;
    if *since_log >= 15.0 {
        *since_log = 0.0;
        let count = |f: fn(&SlotState) -> bool| pool.slots.iter().filter(|s| f(&s.state)).count();
        tracing::debug!(
            target,
            agents = state.world.agents().len(),
            road_km = state.road_length_m / 1000.0,
            hour,
            day_bit,
            time_factors = ?factors,
            wanted = ?wanted,
            alive = ?alive,
            capped,
            ready = count(|s| matches!(s, SlotState::Ready(_))),
            pending = count(|s| matches!(s, SlotState::Pending)),
            failed = count(|s| matches!(s, SlotState::Failed)),
            rejected = ?*rejected,
            "traffic state"
        );
    }
    // A scheduled vehicle asked for by `plan_scheduled` goes first and does not count against the
    // target of the unscheduled traffic.
    let request = sched.request.take();
    let unscheduled = state
        .world
        .agents()
        .len()
        .saturating_sub(sched.agent_count());
    if request.is_none() && unscheduled >= target {
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
    let result = (|| -> Result<_, usize> {
        if let Some(found) = requested {
            return Ok(found);
        }
        let fwd = cam.forward();
        let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
        // The group and vehicle type come first: bus and truck lanes admit only their own, and
        // the lane is drawn by length × the group's density on it.
        let slot = pick_slot(&pool, &deficits, &mut rng).ok_or(0_usize)?;
        let class = match &pool.slots[slot].state {
            SlotState::Ready(v) => v.class(),
            _ => VehicleClass::Car,
        };
        let group = pool.groups[pool.slots[slot].group].density_group;
        let defaults = &pool.defaults;
        let (lane_id, s) = state
            .world
            .graph()
            .weighted_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, &mut rng, |l| {
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
    })();
    let (lane_id, s, slot, at) = match result {
        Ok(found) => found,
        Err(reason) => {
            rejected[reason] += 1;
            pool.rng = Some(rng);
            return;
        }
    };

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
        &mut commands,
        &mut job,
        &mut meshes,
        &mut materials,
        &mut images,
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
            if unusable {
                pool.slots[slot].state = SlotState::Failed;
            } else {
                pool.slots[slot].fails += 1;
                if pool.slots[slot].fails >= MAX_LOAD_TRIES {
                    pool.slots[slot].state = SlotState::Failed;
                }
            }
            pool.rng = Some(rng);
            return;
        }
    };
    place_and_insert_agent(
        &mut commands,
        &mut job,
        &mut state,
        &mut pool,
        &mut sched,
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
