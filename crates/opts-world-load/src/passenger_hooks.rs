//! The passenger rig as the world load sees it (ISS-064): the load attaches a vehicle's rig and
//! joins the units of its chain through the hook this module defines. The rig is a Bevy component
//! with its systems, and those stay in `opts-game`; this crate does not depend on them. The
//! implementation is `opts-game`'s `world_passenger_hooks::GamePassengerHooks`.

use std::sync::Arc;

use bevy::prelude::*;
use opts_core::formats::ticketpack::TicketPack;
use opts_sim::passengers::Unit;

use crate::load_job::WorldLoadJob;

/// The sizes of a rig the load attached, as its log reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RigCounts {
    /// Places of the cabin.
    pub places: usize,
    /// Entries of the cabin.
    pub entries: usize,
    /// Tickets of the map's ticket system.
    pub tickets: usize,
}

/// The calls the world load makes into the passenger rig of `opts-game`. Each method forwards to
/// the rig's own code with the same arguments, in the same order, as the load used to call it
/// directly (ISS-064).
pub trait PassengerHooks: Send + Sync {
    /// `PassengerRig::from_files`, then the insert of the rig on `root`. Returns `None` and inserts
    /// nothing when the cabin has no entry or the paths no pathpoint.
    fn attach_rig(
        &self,
        commands: &mut Commands,
        root: Entity,
        cabin: &[u8],
        paths: &[u8],
        tickets: TicketPack,
        seed: u64,
    ) -> Option<RigCounts>;

    /// `PassengerRig::add_unit` on the rig of `head`, when it has one. The load queues this on the
    /// world, so it runs with the world's access.
    fn add_unit(&self, world: &mut World, head: Entity, seg: Entity, unit: Unit, store: Entity);
}

/// Hooks for a load without a passenger rig: none is attached and no unit joins.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPassengerHooks;

impl PassengerHooks for NoPassengerHooks {
    fn attach_rig(
        &self,
        _commands: &mut Commands,
        _root: Entity,
        _cabin: &[u8],
        _paths: &[u8],
        _tickets: TicketPack,
        _seed: u64,
    ) -> Option<RigCounts> {
        None
    }

    fn add_unit(
        &self,
        _world: &mut World,
        _head: Entity,
        _seg: Entity,
        _unit: Unit,
        _store: Entity,
    ) {
    }
}

impl WorldLoadJob {
    /// Sets the passenger hooks of `opts-game` (ISS-064).
    pub fn set_passenger_hooks(&mut self, hooks: Arc<dyn PassengerHooks>) {
        self.passenger_hooks = hooks;
    }
}
