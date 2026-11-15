//! Resolving a validated formation against the installation and queuing its spawn.

use std::path::PathBuf;

use opts_core::formats::trains::{FormationIssue, Orientation, TrainFormation, VehicleRef};

use crate::in_game::omsi_world::{FormationCar, WorldLoadJob};
use crate::session_vehicles::{EntrySel, SessionVehicles, SpawnParams};

use super::lookup::JobLookup;

/// A validated formation as the spawner needs it: the head's file and the cars behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFormation {
    /// The first car (the player's vehicle).
    pub head: PathBuf,
    /// The cars behind it, in order.
    pub cars: Vec<FormationCar>,
}

/// Validates `formation` and resolves its files.
///
/// # Errors
/// Every [`FormationIssue`] found.
pub fn resolve(
    formation: &TrainFormation,
    job: &WorldLoadJob,
) -> Result<ResolvedFormation, Vec<FormationIssue>> {
    let issues = formation.validate(&JobLookup(job));
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut files = formation.segments.iter().filter_map(|s| match &s.vehicle {
        VehicleRef::File(f) => job
            .bus_file(f)
            .map(|p| (p, s.orientation == Orientation::Backward, s.powered)),
        VehicleRef::Group(_) => None,
    });
    let Some((head, _, _)) = files.next() else {
        return Err(vec![FormationIssue::TooShort]);
    };
    Ok(ResolvedFormation {
        head,
        cars: files
            .map(|(path, turned, powered)| FormationCar {
                path,
                turned,
                powered,
            })
            .collect(),
    })
}

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
