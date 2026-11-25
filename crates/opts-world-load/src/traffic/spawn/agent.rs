//! Spawning traffic agent simulation state, scripts, sounds, and components.

use std::sync::Arc;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_traffic::{Calibration, IdmParams, LaneId, Rng};

use crate::trace;
use crate::traffic::scheduled::ScheduledRequest;
use crate::traffic_variant::SlotState;
use crate::WorldLoadJob;

use super::models::SpawnedModels;
use super::variants::unit_size;
use super::*;

mod log;
use log::{log_spawned, trace_spawned};
mod place;
mod support;
use place::{agent_params, fill_unit_sizes, finish_spawned, Placed};

/// The canvases of the head's script textures, once.
type HeadCanvases =
    Option<Arc<std::sync::RwLock<Vec<opts_core::formats::script_texture::CpuScriptTexture>>>>;

/// The sound set of a vehicle.
type SoundSet = Arc<crate::vehicle_sound::AiSoundSet>;

mod route;
use route::route_scheduled;
mod attach;
use attach::{attach_agent, give_up, AgentLook};
mod look;
use look::{variant_look, VariantLook};

pub(crate) fn place_and_insert_agent(
    (commands, job): (&mut Commands, &mut WorldLoadJob),
    (state, pool, sched): (&mut TrafficState, &mut TrafficPool, &mut ScheduledTraffic),
    (slot, lane_id, s, at): (usize, LaneId, f64, DVec2),
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
        head_canvases,
        head_setvars,
        trace_frame,
    } = models;
    let root = entities[0];
    let SlotState::Ready(variant) = &mut pool.slots[slot].state else {
        return;
    };
    fill_unit_sizes(variant, &bounds);
    let traits = variant.traits();
    let params = agent_params(&traits, (request.is_some(), density_group), rng);
    if state
        .world
        .graph()
        .lane(lane_id)
        .is_some_and(|l| !l.rules.allows(params.class))
    {
        give_up(commands, entities, (sched, request));
        return;
    }
    let speed = match request {
        Some(r) => r.speed.min(state.world.lane_cap(&params, lane_id)),
        None => state.world.lane_cap(&params, lane_id) * 0.8,
    };
    let look = variant_look(variant, &params, &entities);
    let fallbacks = variant.units[0].profile.fallbacks.clone();
    for (unit, &e) in variant.units.iter().zip(&entities) {
        opts_bevy::spawn_vehicle_glows(commands, root, e, &unit.model);
    }
    let mut rng_pax = Rng::new(rng.next_u64());
    match state.world.spawn(lane_id, s, speed, params) {
        Ok(id) => {
            pool.agent_groups.insert(id, pool_group);
            let placed = Placed {
                id,
                lane_id,
                at,
                speed,
                params: &params,
                traits: &traits,
                group_name: &group_name,
                fallbacks: &fallbacks,
            };
            finish_spawned(
                (job, commands, state, sched),
                (&placed, request),
                (&entities, &origin, &trace_frame),
                (look, head_canvases, &head_setvars),
                &mut rng_pax,
            );
        }
        Err(_) => give_up(commands, entities, (sched, request)),
    }
}
