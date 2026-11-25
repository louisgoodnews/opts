//! Queuing the spawn of a resolved formation. The resolution is in `opts-world-load`
//! (`formation`), re-exported here under its old path.

use opts_core::formats::trains::TrainFormation;

pub use opts_world_load::formation::{resolve, ResolvedFormation};

use crate::session_vehicles::{EntrySel, SessionVehicles, SpawnParams};

/// Queues the spawn of a resolved formation at `entry` and returns the session vehicle's id.
pub fn queue_spawn(
    registry: &mut SessionVehicles,
    formation: &TrainFormation,
    resolved: &ResolvedFormation,
    entry: EntrySel,
) -> crate::session_vehicles::VehicleId {
    let stem = resolved
        .head
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    registry.request_spawn(SpawnParams {
        catalogue_id: stem,
        name: formation.name.clone(),
        bus_file: resolved.head.display().to_string(),
        repaint: String::new(),
        setvars: Vec::new(),
        entry,
        formation: Some(formation.clone()),
    })
}
