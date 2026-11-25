//! The plain-data types of the session registry (ISS-064): ids, lifecycle, spawn requests, the
//! vehicle record and its saved state. The registry resource and its systems stay in `opts-game`,
//! which re-exports these types at its old paths.

pub mod state;
pub mod types;

pub use state::VehicleState;
pub use types::{
    DespawnReason, EntrySel, Lifecycle, SessionVehicle, SessionVehicleRecord, SpawnParams,
    SpawnPoint, VehicleId, SAVE_SECTION, SAVE_SECTION_VERSION,
};
