//! The unit and train specs of AI rail vehicles, read from the vehicle files (`.bus`/`.ovh` chains and `.zug` consists).

use super::*;

mod zug;
pub(super) use zug::load_zug_spec;

/// One unit of a consist.
pub(super) struct UnitSpec {
    pub(super) name: String,
    pub(super) lod: ModelLod,
    pub(super) model_dir: std::path::PathBuf,
    pub(super) tex_dirs: Vec<std::path::PathBuf>,
    /// Distance of the unit's origin behind the head unit's origin (m).
    pub(super) behind: f32,
    /// Hangs backwards (coupled with the `true` flag).
    pub(super) reversed: bool,
    /// Half bogie span (`[boogies]`; 0 when unknown).
    pub(super) span: f32,
    /// The unit's footprint (`[boundingbox]`).
    pub(super) body: Option<super::super::BodyBox>,
    /// For the unit's script run.
    pub(super) defs: UnitDefs,
}

/// What a unit's own script run needs: its vehicle file and the text and script textures of its model.
#[derive(Clone)]
pub(super) struct UnitDefs {
    pub(super) bus_path: std::path::PathBuf,
    pub(super) text: Vec<opts_core::formats::vehicles::model::TextTexture>,
    pub(super) script: Vec<opts_core::formats::vehicles::model::ScriptTexture>,
}

/// A trainset assembled from a `.bus`/`.ovh` chain.
pub(super) struct TrainSpec {
    pub(super) name: String,
    pub(super) units: Vec<UnitSpec>,
}

fn parse_unit(
    job: &WorldLoadJob,
    path: &Path,
) -> Option<(
    BusManifest,
    ModelLod,
    std::path::PathBuf,
    Vec<std::path::PathBuf>,
    UnitDefs,
)> {
    let manifest = BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()?;
    let dir = path.parent()?.to_path_buf();
    let cfg = job.install.find_in(&dir, manifest.model_cfg.as_deref()?)?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).ok()?).ok()?;
    let lod = model
        .lods
        .iter()
        .find(|l| l.threshold == 0.0)
        .or_else(|| model.lods.first())?
        .clone();
    let model_dir =
        crate::in_game::omsi_world::mesh_dir::model_mesh_dir(&job.install, &dir, &cfg, &model);
    let tex_dirs = crate::new_game_screen::repaints::texture_dirs(&dir, &model_dir);
    let defs = UnitDefs {
        bus_path: path.to_path_buf(),
        text: model.text_textures.clone(),
        script: model.script_textures.clone(),
    };
    Some((manifest, lod, model_dir, tex_dirs, defs))
}

/// The coupling chain of a `[type] 2` vehicle and whether it is a light one (bicycle, e-scooter:
/// under a tonne; OMSI files them under the rail type too); `None` for road vehicles and
/// unreadable files.
pub(super) fn load_spec(job: &WorldLoadJob, rel: &str) -> Option<(TrainSpec, bool)> {
    let path = job.install.find_file(rel)?;
    let (mut manifest, lod, model_dir, tex_dirs, defs) = parse_unit(job, &path)?;
    if !is_rail(&manifest) {
        return None;
    }
    let light = manifest.mass_tonnes.is_some_and(|m| m < 1.0);
    let name = path.file_stem()?.to_string_lossy().into_owned();
    let mut units = vec![UnitSpec {
        defs: defs.clone(),
        name: name.clone(),
        lod,
        model_dir,
        tex_dirs,
        behind: 0.0,
        reversed: false,
        span: manifest.bogie_half_span.unwrap_or(0.0),
        body: manifest
            .bounding_box
            .map(|(size, centre)| super::super::BodyBox::from_bounding_box(size, centre)),
    }];
    let mut dir = path.parent()?.to_path_buf();
    let mut behind = 0.0_f32;
    for _ in 0..40 {
        let Some(next) = manifest.couple_back.clone() else {
            break;
        };
        let Some(next_path) = job.install.find_in(&dir, &next) else {
            break;
        };
        let Some((m, lod, model_dir, tex_dirs, defs)) = parse_unit(job, &next_path) else {
            break;
        };
        let reversed = manifest.couple_back_reversed;
        let Some((b, f)) = super::super::vehicle::coupling_joint(&manifest, false, &m, reversed)
        else {
            break;
        };
        // The next unit's origin lies at `b − f` in this unit's frame (y forward).
        behind += f.y - b.y;
        units.push(UnitSpec {
            name: next_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            lod,
            model_dir,
            tex_dirs,
            behind,
            reversed,
            span: m.bogie_half_span.unwrap_or(0.0),
            body: m
                .bounding_box
                .map(|(size, centre)| super::super::BodyBox::from_bounding_box(size, centre)),
            defs,
        });
        if reversed {
            break; // the ring of a double-ended consist closes here
        }
        dir = next_path.parent()?.to_path_buf();
        manifest = m;
    }
    Some((TrainSpec { name, units }, light))
}
