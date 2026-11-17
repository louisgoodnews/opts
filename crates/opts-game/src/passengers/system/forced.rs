//! A group sent by hand: they appear beside the first entry, as at a stop.

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::super::crowd::MAX_VISIBLE_WAITING;
use super::super::destinations::draw_dest;
use super::super::rig::{PassengerRig, Unseen, Walker};
use super::PassengerEnv;

pub(super) fn spawn_forced_group(
    e: &mut PassengerEnv,
    rig: &mut PassengerRig,
    frames: &[Affine3A],
    to_world: Affine3A,
) {
    let n = std::mem::take(&mut rig.force_group);
    rig.sim.arrive_at_stop();
    let (door_unit, door) = rig.door_outside(rig.force_entry);
    let frame = frames.get(door_unit).copied().unwrap_or(to_world);
    let side = if door.x >= 0.0 { 1.0 } else { -1.0 };
    for slot in 0..n {
        let dest = draw_dest(rig);
        // Beyond the drawn crowd a group boards unseen, like the crowd of a real stop.
        let actor = (slot < MAX_VISIBLE_WAITING)
            .then(|| {
                e.pool
                    .spawn_actor(&mut e.commands, &mut e.bindposes, rig.rng.next_f32())
            })
            .flatten();
        let Some(actor) = actor else {
            let entry = rig.force_entry;
            rig.unseen.push(Unseen {
                entry,
                age: 18 + (rig.rng.next_f32() * 50.0) as u32,
                stops: 1 + (rig.rng.next_f32() * 3.0) as u32,
                dest,
            });
            continue;
        };
        let local = door
            + Vec3::new(
                side * (1.6 + (slot % 3) as f32 * 0.8),
                0.0,
                ((slot / 3) as f32 - (n / 3) as f32 / 2.0) + (slot % 3) as f32 * 0.35,
            );
        let spot = frame.transform_point3(local);
        let entry = rig.nearest_entry(spot, frames);
        let age = actor.age.unwrap_or(18 + (rig.rng.next_f32() * 50.0) as u32);
        let stops_to_ride = 1 + (rig.rng.next_f32() * 3.0) as u32;
        e.commands
            .entity(actor.entity)
            .insert(Transform::from_translation(spot));
        rig.approaching.push(Walker {
            entity: actor.entity,
            leg_m: actor.leg_m,
            phase: 0.0,
            position: spot,
            target: Vec3::ZERO,
            entry,
            age,
            stops: stops_to_ride,
            walked_m: 0.0,
            dest,
        });
    }
}
