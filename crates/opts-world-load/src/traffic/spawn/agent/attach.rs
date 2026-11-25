//! Attaching the traffic components of a spawned agent to its root and its trailing units.

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_traffic::Rng;

use super::*;
use crate::sound_hooks::SoundHooks;

/// What the vehicle's body and wheels need to follow its agent.
pub(super) struct AgentLook {
    pub(super) delta_height: f32,
    pub(super) wheel_radius: f32,
    pub(super) rear_axle_y: f32,
    pub(super) at: DVec2,
}

/// The traffic components of the vehicle's root, its sounds and its trailing units.
pub(super) fn attach_agent(
    commands: &mut Commands,
    hooks: &dyn SoundHooks,
    (root, entities): (Entity, &[Entity]),
    (id, rng_pax): (AgentId, &mut Rng),
    look: AgentLook,
    (sound, unit_sounds, scripted_head, chain): (
        Option<&SoundSet>,
        &[Option<SoundSet>],
        bool,
        Vec<ChainUnit>,
    ),
) {
    let AgentLook {
        delta_height,
        wheel_radius,
        rear_axle_y,
        at,
    } = look;
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
    hooks.attach_ai(commands, root, sound);
    for (set, &e) in unit_sounds.iter().zip(entities.iter().skip(1)) {
        let Some(set) = set else {
            continue;
        };
        hooks.attach_ai_unit(commands, e, Some(root), set);
        if scripted_head {
            hooks.attach_scripted_defs(commands, e, Some(root), &set.scripted);
        }
    }
    if !chain.is_empty() {
        commands.entity(root).insert(TrafficChain { units: chain });
    }
}

/// A spawn that did not come to be: the unit roots go and a scheduled run is told.
pub(super) fn give_up(
    commands: &mut Commands,
    entities: Vec<Entity>,
    (sched, request): (&mut ScheduledTraffic, Option<&ScheduledRequest>),
) {
    for e in entities {
        commands.entity(e).despawn();
    }
    if let Some(r) = request {
        sched.failed(r);
    }
}
