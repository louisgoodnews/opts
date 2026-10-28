//! Spawning traffic agent simulation state, scripts, sounds, and components.

use std::sync::Arc;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_traffic::{Calibration, IdmParams, LaneId, Rng, VehicleClass};

use crate::in_game::omsi_world::trace;
use crate::in_game::omsi_world::traffic::scheduled::ScheduledRequest;
use crate::in_game::omsi_world::traffic_variant::SlotState;
use crate::in_game::omsi_world::WorldLoadJob;

use super::models::SpawnedModels;
use super::variants::unit_size;
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn place_and_insert_agent(
    commands: &mut Commands,
    job: &mut WorldLoadJob,
    state: &mut TrafficState,
    pool: &mut TrafficPool,
    sched: &mut ScheduledTraffic,
    slot: usize,
    lane_id: LaneId,
    s: f64,
    at: DVec2,
    request: Option<&ScheduledRequest>,
    models: SpawnedModels,
    rng: &mut Rng,
) {
    let pool_group = pool.slots[slot].group;
    let group_name = pool.groups[pool_group].name.clone();
    let density_group = u8::try_from(pool.groups[pool_group].density_group).unwrap_or(u8::MAX);
    let SpawnedModels {
        entities,
        bounds,
        origin,
        mut head_canvases,
        head_setvars,
        trace_frame,
    } = models;
    let root = entities[0];
    let SlotState::Ready(variant) = &mut pool.slots[slot].state else {
        return;
    };
    for (unit, (min, max)) in variant.units.iter_mut().zip(&bounds) {
        let boxed = unit.profile.body_length_m;
        unit.size
            .get_or_insert_with(|| unit_size(min.z, max.z, boxed));
    }
    let traits = variant.traits();
    let mut params = IdmParams::from_traits(&traits, &Calibration::default(), rng.next_f64());
    params.density_group = if request.is_some() {
        u8::MAX
    } else {
        density_group
    };
    if state
        .world
        .graph()
        .lane(lane_id)
        .is_some_and(|l| !l.rules.allows(params.class))
    {
        for e in entities {
            commands.entity(e).despawn();
        }
        if let Some(r) = request {
            sched.failed(r);
        }
        return;
    }
    let speed = match request {
        Some(r) => r.speed.min(state.world.lane_cap(&params, lane_id)),
        None => state.world.lane_cap(&params, lane_id) * 0.8,
    };
    let head = &variant.units[0];
    let sound = head.sound.clone();
    let door_ahead = head
        .front_door_y
        .map_or(0.0, |y| (head.size.unwrap_or((4.4, 2.2)).1 - y).max(0.0));
    let unit_sounds: Vec<Option<Arc<crate::vehicle_sound::AiSoundSet>>> = variant
        .units
        .iter()
        .skip(1)
        .map(|u| u.sound.clone())
        .collect();
    let size = head.size.unwrap_or((4.4, 2.2));
    let heavy = params.class != VehicleClass::Car;
    let wheel_radius = head
        .profile
        .wheel_radius_m
        .unwrap_or(if heavy { 0.5 } else { 0.31 });
    let rear_axle_y = size.1 - (params.front_overhang + params.wheelbase) as f32;
    let delta_height = head.profile.delta_height_m;
    let name = variant.name.clone();
    let fallbacks = head.profile.fallbacks.clone();
    let measured = head.response.is_some();
    let head_bus_path = head.bus_path.clone();
    let chain: Vec<ChainUnit> = variant
        .units
        .iter()
        .zip(&entities)
        .skip(1)
        .map(|(u, &e)| {
            ChainUnit::new(
                e,
                u.hitch,
                u.joint,
                u.rear_axle_y,
                u.max_alpha_deg,
                u.reversed,
            )
        })
        .collect();
    for (unit, &e) in variant.units.iter().zip(&entities) {
        opts_bevy::spawn_vehicle_glows(commands, root, e, &unit.model);
    }
    let mut rng_pax = Rng::new(rng.next_u64());
    let mut scripted_head = false;
    match state.world.spawn(lane_id, s, speed, params) {
        Ok(id) => {
            pool.agent_groups.insert(id, pool_group);
            if let Some(r) = request {
                state.world.set_route(id, r.route.iter().copied());
                for stop in &r.stops {
                    let mut stop = *stop;
                    let room = state
                        .world
                        .graph()
                        .lane(stop.lane)
                        .map_or(stop.s, |l| (l.length() - 0.5).max(0.0));
                    stop.s = (stop.s + f64::from(door_ahead)).min(room);
                    state.world.push_stop(id, stop);
                }
                sched.built(id, r);
                if let Some(canvases) = head_canvases.take() {
                    let seed = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ r.run.run as u64;
                    let scripted = super::super::ai_script::attach(
                        job,
                        commands,
                        root,
                        &head_bus_path,
                        canvases,
                        &r.run,
                        seed,
                        &head_setvars,
                    );
                    if let (true, Some(set)) = (scripted, sound.as_ref()) {
                        crate::player_sound::attach_defs(commands, root, None, &set.scripted);
                    }
                    scripted_head = scripted;
                }
            }
            tracing::debug!(
                agent = id.0,
                vehicle = %name,
                group = %group_name,
                speed,
                mass_kg = traits.mass_kg,
                desired_kmh = params.desired_speed * 3.6,
                accel = params.max_accel,
                comfort = params.comfort_decel,
                max_decel = params.max_decel,
                class = ?params.class,
                units = entities.len(),
                measured,
                fallbacks = ?fallbacks,
                "AI vehicle spawned"
            );
            if opts_bevy::scene_trace::enabled() {
                trace::spawned(
                    opts_bevy::scene_trace::kind::TRAFFIC,
                    &name,
                    &name,
                    root,
                    &origin,
                    &trace_frame,
                    "ok",
                    serde_json::json!({
                        "lane": format!("{lane_id:?}"),
                        "speed": speed,
                        "class": format!("{:?}", params.class),
                        "units": entities.len(),
                    }),
                );
            }
            let mut pax = 0u8;
            for (i, p) in [0.9, 0.35, 0.2, 0.1].into_iter().enumerate() {
                if rng_pax.next_f64() < p {
                    pax |= 1 << i;
                }
            }
            commands.entity(root).insert((
                TrafficVehicle {
                    id,
                    delta_height,
                    wheel_radius,
                    rear_axle_y,
                    wheel_angle: 0.0,
                    pax,
                    blink_phase: rng_pax.next_f64() as f32 * 0.7,
                    origin: at,
                    heading: 0.0,
                    height: delta_height,
                    honk_after: HONK_AFTER_S,
                },
                opts_bevy::AiVariables::default(),
            ));
            crate::vehicle_sound::attach_ai(commands, root, sound.as_ref());
            for (set, &e) in unit_sounds.iter().zip(entities.iter().skip(1)) {
                let Some(set) = set else {
                    continue;
                };
                crate::vehicle_sound::attach_ai_unit(commands, e, Some(root), set);
                if scripted_head {
                    crate::player_sound::attach_defs(commands, e, Some(root), &set.scripted);
                }
            }
            if !chain.is_empty() {
                commands.entity(root).insert(TrafficChain { units: chain });
            }
        }
        Err(_) => {
            for e in entities {
                commands.entity(e).despawn();
            }
            if let Some(r) = request {
                sched.failed(r);
            }
        }
    }
}
