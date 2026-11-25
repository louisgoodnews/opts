//! Picking a place for a new vehicle and finishing its spawn.

use super::*;

/// Picks a group's vehicle type and a lane for it in the ring around the camera. The error says
/// why no place was found: no vehicle ready (or no group below its target), no lane in the ring,
/// lane gone, in the camera's view.
pub(super) fn pick_place(
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
pub(super) fn finish_spawn(
    (commands, job): (&mut Commands, &mut WorldLoadJob),
    (state, pool, sched): (&mut TrafficState, &mut TrafficPool, &mut ScheduledTraffic),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (chrono, request): (
        Option<&crate::chrono::ChronoState>,
        Option<super::super::scheduled::ScheduledRequest>,
    ),
    ((lane_id, s, slot, at), mut rng): ((opts_traffic::LaneId, f64, usize, DVec2), Rng),
) {
    pool.built += 1;
    let repaint_seed = opts_repaints::instance_seed(&[slot as u64, pool.built]);
    let SlotState::Ready(variant) = &pool.slots[slot].state else {
        pool.rng = Some(rng);
        if let Some(r) = &request {
            sched.failed(r);
        }
        return;
    };
    let fleet = &pool.slots[slot].fleet;
    let models = match build_unit_models(
        (commands, job),
        (meshes, materials, images),
        (variant, at, fleet),
        (repaint_seed, chrono, request.is_some()),
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
        (commands, job),
        (state, pool, sched),
        (slot, lane_id, s, at),
        request.as_ref(),
        models,
        &mut rng,
    );
    pool.rng = Some(rng);
}
