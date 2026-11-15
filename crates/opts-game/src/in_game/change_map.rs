//! Changing the map inside a running drive (M-0108).
//!
//! The Alt menu's *Change map* page and the control protocol's `session.change_map` both end
//! here: the rules of what is carried over to the other map are a pure function ([`plan`]), and
//! [`start`] writes the plan into the session setup and enters the session again through the
//! same teardown and loading path as *Load game* (`save::restore::begin_restore`), so no second
//! loading path exists.
//!
//! Carry-over: the active vehicle (same catalogue id, repaint and equipment) is built again at an
//! entry point of the target map; the session's date, time of day and weather are kept; the
//! other session vehicles are dropped and listed in the plan; a session without a vehicle stays
//! without one. Where the vehicle cannot be carried (it is not in the catalogue any more) the
//! plan fails with [`ChangeMapError::VehicleUnavailable`], and the caller asks the player: choose
//! another vehicle from the menu first, or [`VehicleChoice::None`] to go on without one.

use bevy::prelude::*;
use opts_bevy::environment::{CalendarDate, SessionEnvironment};

use crate::app_state::AppState;
use crate::new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem};
use crate::session_vehicles::{SessionVehicleRecord, SessionVehicles};

#[cfg(test)]
mod tests;

mod find;
mod plan;
mod session;
mod types;

pub use find::*;
pub use plan::*;
pub use session::*;
pub use types::*;
