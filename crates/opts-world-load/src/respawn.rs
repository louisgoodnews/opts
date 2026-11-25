//! Spawning session vehicles while a session runs (the in-game vehicle menu, the control protocol):
//! the entry points of the map resolved to world positions, and the spawner that works down the
//! spawn requests of the [`SessionVehicles`] registry and builds each vehicle at its entry point.
//! Despawning, switching and respawning are in `crate::session_vehicles::ops`.

use super::vehicle::load_player_vehicle;
use super::{LoadAssets, WorldLoadJob, WorldStats};
use opts_game_state::session_vehicles::{SessionVehicles, SpawnPoint, VehicleId};
use bevy::prelude::*;
use opts_bevy::{CameraMode, CameraState};
use opts_core::formats::map::{MapEntryPoint, MapTile};
use opts_gui::l10n::Localization;
use opts_gui::prelude::ToastBuilder;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod entry_index;
pub use entry_index::{EntryIndex, ResolvedEntry};

mod flight;
mod place;
mod request;

use flight::{complete_flight, join_issues};
use place::{place_at_entry, place_at_stop};
use request::VehicleFilter;
pub use request::{handle_vehicle_requests, SpawnVehicleRequest};
