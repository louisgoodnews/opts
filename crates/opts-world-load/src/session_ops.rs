//! The session-vehicle layer as the world load sees it (ISS-064): the operations on the
//! `SessionVehicles` resource that the load reaches, and the hook through which it calls them. The
//! registry resource and its operations stay in `opts-game`; this crate does not depend on them.

use std::sync::Arc;

use bevy::prelude::*;
use opts_game_state::session_vehicles::VehicleId;

use crate::load_job::WorldLoadJob;

/// The calls the world load makes into the session-vehicle operations of `opts-game`. Each method
/// forwards to one function there with the same arguments and in the same order as the load used to
/// call it directly (ISS-064). The implementation is `opts-game`'s `world_session_ops`.
pub trait SessionVehicleOps: Send + Sync {
    /// `session_vehicles::ops::switch_to`: makes vehicle `id` the active one. The error is the text
    /// of the operation's error.
    fn switch_to(&self, world: &mut World, id: VehicleId) -> Result<(), String>;
}

/// Operations that do nothing but fail. A load job starts with these and `opts-game` replaces them
/// before the job is used; code that runs a job without `opts-game` keeps them.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSessionVehicleOps;

impl SessionVehicleOps for NoSessionVehicleOps {
    fn switch_to(&self, _world: &mut World, _id: VehicleId) -> Result<(), String> {
        Err("the session vehicles are not available".to_owned())
    }
}

impl WorldLoadJob {
    /// The session-vehicle operations the job calls.
    pub fn session_ops(&self) -> Arc<dyn SessionVehicleOps> {
        self.session_ops.clone()
    }

    /// Installs the session-vehicle operations the job calls. `opts-game` does this before the job is
    /// used.
    pub fn set_session_ops(&mut self, ops: Arc<dyn SessionVehicleOps>) {
        self.session_ops = ops;
    }
}
