//! The passenger rig's side of the world load (ISS-064): `opts-world-load` attaches a vehicle's rig
//! and joins the units of its chain through `PassengerHooks`, and this forwards each call to
//! `PassengerRig` with the same arguments.

use bevy::prelude::*;
use opts_core::formats::ticketpack::TicketPack;
use opts_sim::passengers::Unit;
use opts_world_load::passenger_hooks::{PassengerHooks, RigCounts};

use crate::passengers::PassengerRig;

/// The passenger hooks of the game: `passengers::PassengerRig`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GamePassengerHooks;

impl PassengerHooks for GamePassengerHooks {
    fn attach_rig(
        &self,
        commands: &mut Commands,
        root: Entity,
        cabin: &[u8],
        paths: &[u8],
        tickets: TicketPack,
        seed: u64,
    ) -> Option<RigCounts> {
        let rig = PassengerRig::from_files(cabin, paths, tickets, seed, root)?;
        let counts = RigCounts {
            places: rig.sim.cabin.places.len(),
            entries: rig.sim.cabin.entries.len(),
            tickets: rig.sim.tickets.tickets.len(),
        };
        commands.entity(root).insert(rig);
        Some(counts)
    }

    fn add_unit(&self, world: &mut World, head: Entity, seg: Entity, unit: Unit, store: Entity) {
        if let Some(mut rig) = world.get_mut::<PassengerRig>(head) {
            rig.add_unit(seg, unit, store);
        }
    }
}
