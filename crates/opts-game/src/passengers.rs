//! The passengers of the player's vehicle in the world (M-0101).
//!
//! The logic is [`opts_sim::passengers`] (the cabin, the doors, the tickets); this module is its
//! place in the game: it finds the bus stop the bus stands at, lets people walk up to the open
//! doors and in, shows them inside (walking, seated or standing), lets them get off and walk
//! away, and tells the vehicle's scripts how many people there are and which doors are wanted
//! (`humans_count`, `PAX_Entry<n>_Req`, `PAX_Exit<n>_Req`).
//!
//! Doors are the scripts' `PAX_Entry<n>_Open` and `PAX_Exit<n>_Open` (the n-th `[entry]` and
//! `[exit]` of the cabin). Specification: `docs/src/simulation/passengers.md`.

use bevy::prelude::*;

use crowd::StopCrowds;

/// Key of the setting that turns the passengers on and off.
pub const SETTING: &str = "gameplay.passengers";
/// The bus counts as being at a stop within this distance of the marker (m).
pub(crate) const AT_STOP_M: f32 = 22.0;
/// …and slower than this (km/h).
pub(crate) const STOPPED_KMH: f32 = 3.0;
/// The bus has left when faster than this (km/h) or farther than [`LEFT_M`] from the stop.
pub(crate) const MOVING_KMH: f32 = 6.0;
/// Distance at which a stop is left (m).
pub(crate) const LEFT_M: f32 = 45.0;
/// Walking speed outside (m/s).
pub(crate) const OUTSIDE_SPEED_M_S: f32 = 1.3;
/// How far a passenger who got off walks away before they vanish (m).
pub(crate) const LEAVE_DISTANCE_M: f32 = 9.0;

/// Face `direction` (a horizontal vector) as a rotation about the vertical.
pub(crate) fn facing(direction: Vec3) -> Quat {
    let flat = Vec3::new(direction.x, 0.0, direction.z);
    if flat.length_squared() < 1e-6 {
        Quat::IDENTITY
    } else {
        Transform::IDENTITY.looking_to(flat, Vec3::Y).rotation
    }
}

mod convert;
mod crowd;
mod draw;
mod feed;
mod protocol;
mod rig;
mod system;
mod weight;
pub(crate) use weight::PassengerCountOverride;

pub use convert::{cabin_spec, paths_spec, ticket_book, unit_spec};
pub use protocol::{command, register};
pub use rig::PassengerRig;
pub use system::update_passengers;

/// Registers the passenger system.
pub struct PassengersPlugin;

impl Plugin for PassengersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_passengers, weight::apply_passenger_mass)
                .chain()
                .run_if(in_state(crate::app_state::AppState::InGame)),
        )
        .add_systems(
            OnEnter(crate::app_state::AppState::InGame),
            weight::clear_override,
        )
        .init_resource::<StopCrowds>()
        .init_resource::<weight::PassengerCountOverride>();
    }
}
