//! Vehicle lookup through the installation: a car's manifest and its `[couple_back]` chain.

use std::path::Path;

use opts_core::formats::trains::{VehicleInfo, VehicleLookup};
use opts_core::formats::vehicles::BusManifest;

use crate::in_game::omsi_world::WorldLoadJob;

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
