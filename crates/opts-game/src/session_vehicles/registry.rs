//! The registry of the vehicles the user spawned in a session ([`SessionVehicles`]).
//!
//! A session is a map plus zero or more *user* vehicles. Every vehicle the user brings in (the
//! one chosen in the wizard, the ones spawned later from the pause menu or through the control
//! protocol) gets a stable session id and a [`SessionVehicleRecord`]; parked and AI vehicles are
//! not part of it. The registry is plain data (it serialises as one section of the later session
//! save, [`SAVE_SECTION`]); the live entities are runtime-only fields that the systems of this
//! module keep in step. The lifecycle and the switch and despawn rules are in
//! `docs/src/architecture/session-vehicles.md`.

mod lifecycle;
mod section;
mod store;
mod types;

pub use store::SessionVehicles;
pub use types::{
    DespawnReason, EntrySel, Lifecycle, SessionVehicle, SessionVehicleRecord, SpawnParams,
    SpawnPoint, VehicleId, SAVE_SECTION, SAVE_SECTION_VERSION,
};

#[cfg(test)]
mod tests;
