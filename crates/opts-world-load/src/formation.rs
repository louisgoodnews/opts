//! Validating a train formation through the installation and resolving its vehicle files. The
//! lookup and the resolution read the loader's job only; queuing the spawn stays in `opts-game`
//! (`formation::resolve`), which re-exports these.

use std::path::{Path, PathBuf};

use opts_core::formats::trains::{
    FormationIssue, Orientation, TrainFormation, VehicleInfo, VehicleLookup, VehicleRef,
};
use opts_core::formats::vehicles::BusManifest;

use crate::{FormationCar, WorldLoadJob};

/// Looks vehicles up through the installation of the running session.
pub struct JobLookup<'a>(pub &'a WorldLoadJob);

impl JobLookup<'_> {
    fn manifest(path: &Path) -> Option<BusManifest> {
        BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()
    }
}

impl VehicleLookup for JobLookup<'_> {
    fn info(&self, file: &str) -> Option<VehicleInfo> {
        let path = self.0.bus_file(file)?;
        let head = Self::manifest(&path)?;
        let mut info = VehicleInfo::from_manifest(&head);
        // Follow the car's own `[couple_back]` chain as the spawner does: a link flagged
        // reversed leads back into the chain and ends it, with the last file turned around, so
        // the car's rear coupling is that file's front point.
        let (mut dir, mut last) = (path.parent()?.to_path_buf(), head);
        let mut ring_closed = false;
        for _ in 0..opts_core::formats::trains::MAX_PARTS {
            let Some(next) = last.couple_back.clone() else {
                break;
            };
            let Some(p) = self.0.find_vehicle_in(&dir, &next) else {
                break;
            };
            let Some(m) = Self::manifest(&p) else { break };
            info.parts += 1;
            ring_closed = last.couple_back_reversed;
            dir = p.parent()?.to_path_buf();
            last = m;
            if ring_closed {
                break;
            }
        }
        info.back_coupling = if ring_closed {
            last.coupling_front.is_some()
        } else {
            last.coupling_back.is_some()
        };
        Some(info)
    }
}

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

