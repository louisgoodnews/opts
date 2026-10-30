//! The steps of placing one AI vehicle once its agent exists.

use super::*;

/// The spawned agent: where it stands and what it was made from.
pub(super) struct Placed<'a> {
    pub(super) id: AgentId,
    pub(super) lane_id: LaneId,
    pub(super) at: DVec2,
    pub(super) speed: f64,
    pub(super) params: &'a IdmParams,
    pub(super) traits: &'a opts_traffic::VehicleTraits,
    pub(super) group_name: &'a str,
    pub(super) fallbacks: &'a dyn std::fmt::Debug,
}

/// The unit sizes the variant lacks, measured from the model bounds.
pub(super) fn fill_unit_sizes(
    variant: &mut crate::in_game::omsi_world::traffic_variant::Variant,
    bounds: &[(Vec3, Vec3)],
) {
    for (unit, (min, max)) in variant.units.iter_mut().zip(bounds) {
        let boxed = unit.profile.body_length_m;
        unit.size
            .get_or_insert_with(|| unit_size(min.z, max.z, boxed));
    }
}

/// The driving parameters of a new vehicle; scheduled ones are outside the density groups.
pub(super) fn agent_params(
    traits: &opts_traffic::VehicleTraits,
    (scheduled, density_group): (bool, u8),
    rng: &mut Rng,
) -> IdmParams {
    let mut params = IdmParams::from_traits(traits, &Calibration::default(), rng.next_f64());
    params.density_group = if scheduled { u8::MAX } else { density_group };
    params
}

/// Routes, logs and decorates the vehicle of a freshly spawned agent.
pub(super) fn finish_spawned(
    (job, commands, state, sched): (
        &mut WorldLoadJob,
        &mut Commands,
        &mut TrafficState,
        &mut ScheduledTraffic,
    ),
    (placed, request): (&Placed, Option<&ScheduledRequest>),
    (entities, origin, trace_frame): (&[Entity], &Transform, &trace::Frame),
    (look, mut head_canvases, head_setvars): (VariantLook, HeadCanvases, &[(String, String)]),
    rng_pax: &mut Rng,
) {
    let Placed {
        id,
        lane_id,
        at,
        speed,
        params,
        traits,
        group_name,
        fallbacks,
    } = *placed;
    let VariantLook {
        sound,
        door_ahead,
        unit_sounds,
        wheel_radius,
        rear_axle_y,
        delta_height,
        name,
        measured,
        head_bus_path,
        chain,
    } = look;
    let root = entities[0];
    let mut scripted_head = false;
    if let Some(r) = request {
        scripted_head = route_scheduled(
            (job, commands, state, sched),
            (id, r),
            (root, door_ahead),
            (
                &mut head_canvases,
                &head_bus_path,
                head_setvars,
                sound.as_ref(),
            ),
        );
    }
    log_spawned(
        id,
        (&name, group_name),
        speed,
        (params, traits),
        (entities.len(), measured, fallbacks),
    );
    trace_spawned(
        root,
        &name,
        origin,
        trace_frame,
        (lane_id, speed, params, entities.len()),
    );
    attach_agent(
        commands,
        (root, entities),
        (id, rng_pax),
        AgentLook {
            delta_height,
            wheel_radius,
            rear_axle_y,
            at,
        },
        (sound.as_ref(), &unit_sounds, scripted_head, chain),
    );
}
