//! The cars of a `.zug` consist: each car's unit spec, joined to the one before by its coupling points.

use super::*;

/// The cars of a `.zug` consist, front to back, each joined to the one before it by their
/// coupling points; a car without them ends the train. Group names (resolved by the map) are
/// skipped. `None` when no car is a readable rail vehicle.
pub(in super::super) fn load_zug_spec(job: &WorldLoadJob, rel: &str) -> Option<(TrainSpec, bool)> {
    use opts_core::formats::trains::{Orientation, TrainFormation, VehicleRef};
    let path = job.install.find_file(rel)?;
    let name = path.file_stem()?.to_string_lossy().into_owned();
    let (formation, _) = TrainFormation::from_zug(&std::fs::read(&path).ok()?, &name, rel).ok()?;
    let mut units: Vec<UnitSpec> = Vec::new();
    let mut prev: Option<(BusManifest, bool)> = None;
    let mut behind = 0.0_f32;
    let mut light = false;
    for seg in &formation.segments {
        let VehicleRef::File(file) = &seg.vehicle else {
            continue;
        };
        let Some(car_path) = job.install.find_file(file) else {
            break;
        };
        let Some((m, lod, model_dir, tex_dirs, defs)) = parse_unit(job, &car_path) else {
            break;
        };
        if !is_rail(&m) {
            break;
        }
        let turned = seg.orientation == Orientation::Backward;
        if let Some((pm, pturned)) = &prev {
            let Some((b, f)) =
                super::super::super::vehicle::coupling_joint(pm, *pturned, &m, turned)
            else {
                break;
            };
            behind += f.y - b.y;
        } else {
            light = m.mass_tonnes.is_some_and(|t| t < 1.0);
        }
        units.push(UnitSpec {
            name: car_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            lod,
            model_dir,
            tex_dirs,
            behind,
            reversed: turned,
            span: m.bogie_half_span.unwrap_or(0.0),
            body: m.bounding_box.map(|(size, centre)| {
                super::super::super::BodyBox::from_bounding_box(size, centre)
            }),
            defs,
        });
        prev = Some((m, turned));
    }
    // Very long trains cost more than the traffic they add.
    (!units.is_empty() && units.len() <= 12).then_some((TrainSpec { name, units }, light))
}
