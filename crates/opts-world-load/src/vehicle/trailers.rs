//! Trailer chains and consist masses.

use super::segments::{load_segment_meshes_for, repaint_map};
use super::*;

mod finish;
mod follower;

pub use opts_consist_plan::FormationCar;

/// The most vehicle files one spawn follows: a single vehicle's `[couple_back]` chain, or a
/// formation's cars with their own chains.
pub(super) fn max_parts(formation: bool) -> usize {
    if formation {
        opts_core::formats::trains::MAX_PARTS
    } else {
        40
    }
}

pub(crate) use opts_consist_plan::coupling_joint;

/// The installation as the planner reads it.
pub(super) struct JobFiles<'a>(&'a WorldLoadJob);

impl super::super::consist_plan::Files for JobFiles<'_> {
    fn find_in(&self, base: &Path, rel: &str) -> Option<PathBuf> {
        self.0.install.find_in(base, rel)
    }

    fn manifest(&self, path: &Path) -> Option<BusManifest> {
        BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()
    }
}

/// Spawns the units behind the head `root`: its `[couple_back]` chain, then the cars of a train
/// formation (`job.formation_cars`), each with its own chain, where `consist_plan` puts them.
/// Camera presets with the unit entity they are defined in.
pub(crate) type CabCameras =
    Vec<(Entity, opts_core::formats::vehicles::bus::BusCamera)>;

/// Returns the passenger and the driver camera presets of all trailers with the segment entity
/// they are defined in.
pub(crate) fn spawn_trailer_chain(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    (bus_dir, tractor): (&Path, &BusManifest),
) -> (CabCameras, CabCameras) {
    let formation: Vec<FormationCar> = std::mem::take(&mut job.formation_cars);
    let rail = super::super::is_rail(tractor);
    let plan = super::super::consist_plan::plan(
        bus_dir,
        tractor,
        &formation,
        max_parts(!formation.is_empty()),
        &JobFiles(job),
    );
    let mut st = finish::new_state(root, tractor);
    for part in &plan.followers {
        if !follower::spawn_follower(
            job,
            commands,
            (&mut *meshes, &mut *materials, &mut *images),
            root,
            (part, rail),
            &mut st,
        ) {
            break;
        }
    }
    finish::finish_chain(commands, root, tractor, rail, &mut st);
    (st.cams, st.driver_cams)
}

/// Mass the simulation assumes for a head unit that states none (kg).
const HEAD_MASS_DEFAULT_KG: f32 = 11_500.0;

/// The mass a vehicle file states (kg); `None` when it states none or an implausible one.
pub(super) fn manifest_mass_kg(manifest: &BusManifest) -> Option<f32> {
    manifest
        .mass_tonnes
        .filter(|&m| m > 0.5)
        .map(|m| m * 1_000.0)
}

/// What a following unit adds to the consist: its stated mass, else `previous_kg` (the unit in
/// front; the installed consists have a median trailer/head ratio of 0.90, `consist_census`), and
/// its axles with those marked driven (`achse_antrieb`).
#[cfg(test)]
pub(super) fn unit_load(
    manifest: &BusManifest,
    reversed: bool,
    previous_kg: f32,
) -> opts_sim::UnitLoad {
    opts_sim::UnitLoad::from_manifest_or(manifest, reversed, previous_kg)
}

/// Mass of a rail consist for the dynamics (kg): the head unit's mass, at least 20 t (the other
/// units usually state 0; assumed, see `docs/src/simulation/rail-vehicles.md`).
pub(super) fn commands_mass(manifest: &BusManifest) -> f32 {
    (manifest.mass_tonnes.unwrap_or(0.0) * 1000.0).max(20_000.0)
}
