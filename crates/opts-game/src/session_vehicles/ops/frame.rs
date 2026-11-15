//! The shared ground of the session operations: the error type, the place-label distance, the
//! position conversions and the vehicle lookups.

use bevy::prelude::*;
use opts_bevy::VehicleController;

use crate::session_vehicles::registry::{Lifecycle, SessionVehicle, SessionVehicles, VehicleId};
use crate::in_game::omsi_world::WorldStats;

/// Why an operation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpError {
    /// No session vehicle has this id.
    NotFound(String),
    /// The vehicle exists but is not in the world (still spawning, failed, gone).
    NotReady(String),
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m) | Self::NotReady(m) => f.write_str(m),
        }
    }
}

/// How far the vehicle's place label stays on screen: once the vehicle is further than this from
/// its spawn point the label is gone for good (until a respawn or a new spawn).
pub const PLACE_LABEL_HIDE_DISTANCE_M: f32 = 25.0;

/// Absolute OMSI position of a point of the session frame (the chassis position).
#[must_use]
pub fn local_to_abs(world: &World, local: Vec2) -> (f64, f64) {
    let (start, grid) = world
        .get_resource::<WorldStats>()
        .map_or(((0, 0), Default::default()), |s| (s.start_tile, s.grid));
    let abs = grid.local_to_absolute(local.as_dvec2(), start);
    (abs.x, abs.y)
}

/// The session-frame position of an absolute OMSI position.
#[must_use]
pub fn abs_to_local(world: &World, x: f64, y: f64) -> Vec2 {
    let (start, grid) = world
        .get_resource::<WorldStats>()
        .map_or(((0, 0), Default::default()), |s| (s.start_tile, s.grid));
    grid.absolute_to_local(bevy::math::DVec2::new(x, y), start)
        .as_vec2()
}

pub(super) fn alive_entity(world: &World, id: VehicleId) -> Result<Entity, OpError> {
    let reg = world.resource::<SessionVehicles>();
    let rec = reg
        .get(id)
        .ok_or_else(|| OpError::NotFound(format!("no session vehicle {id}")))?;
    match (rec.lifecycle, rec.entity) {
        (Lifecycle::Alive, Some(e)) if world.get_entity(e).is_ok() => Ok(e),
        (Lifecycle::Spawning, _) => Err(OpError::NotReady(format!(
            "session vehicle {id} is still being spawned"
        ))),
        (l, _) => Err(OpError::NotReady(format!(
            "session vehicle {id} is not in the world ({l:?})"
        ))),
    }
}

/// Parks a vehicle that stops being the active one: pedals released, parking brake on. The
/// physics brings it to rest.
pub(super) fn park(world: &mut World, entity: Entity) {
    if let Some(mut c) = world.get_mut::<VehicleController>(entity) {
        let inputs = c.inputs_mut();
        inputs.set_throttle(0.0);
        inputs.set_service_brake(0.0);
        inputs.set_steering(0.0);
        inputs.set_parking_brake(true);
    }
}

/// Entity of the session vehicle with this id, if it is in the world.
#[must_use]
pub fn entity_of(world: &World, id: VehicleId) -> Option<Entity> {
    alive_entity(world, id).ok()
}

/// The id a vehicle entity carries.
#[must_use]
pub fn id_of(world: &World, entity: Entity) -> Option<VehicleId> {
    world.get::<SessionVehicle>(entity).map(|s| s.0)
}
