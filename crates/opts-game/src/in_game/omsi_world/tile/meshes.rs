//! Loading scenery-object mesh parts and static/parked vehicle model configs on a tile.

use std::path::PathBuf;

use super::*;
use opts_bevy::ModelPart;
use opts_core::formats::scenery::SceneryObject;

pub(super) type CrossingBend = (
    PathBuf,
    std::sync::Arc<opts_core::formats::scenery::BaseSurface>,
);

mod overrides;
mod vehicle_model;
use overrides::texture_overrides;
pub(super) use vehicle_model::spawn_vehicle_model;

/// Loads the mesh parts of a scenery object's `[mesh]` records; `None` when a required `.o3d` file
/// is missing.
pub(super) fn load_sco_meshes(
    job: &mut WorldLoadJob,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (sco, sco_path, model_dir, tex_dirs): (&SceneryObject, &Path, &Path, &[PathBuf]),
    (obj, key, string_names, bend): (
        &MapObjectInstance,
        (i32, i32),
        &[String],
        Option<&CrossingBend>,
    ),
) -> Option<Vec<Vec<ModelPart>>> {
    let mut mesh_parts = Vec::with_capacity(sco.meshes.len());
    for (mesh_index, mesh_def) in sco.meshes.iter().enumerate() {
        // `[onlyeditor]` after a `[mesh]` hides that mesh only (helpers next to a visible one).
        if !sco.mesh_in_game(mesh_index) {
            mesh_parts.push(Vec::new());
            continue;
        }
        let found = job.install.find_in(model_dir, &mesh_def.file);
        if let Some(p) = &found {
            job.loading_deps
                .insert(opts_bevy::hot_reload::cache::normalize(p));
            super::super::trace::mesh_used(p);
        }
        let Some(o3d_path) = found else {
            job.cache
                .stats
                .failures
                .push(format!("o3d not found: {}", mesh_def.file));
            tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), mesh = %mesh_def.file, "o3d not found");
            super::super::trace::warn(|| format!("o3d not found: {}", mesh_def.file));
            return None;
        };
        let overrides = texture_overrides(
            job,
            images,
            (sco, mesh_def),
            (sco_path, obj, key, string_names),
        );
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        mesh_parts.push(loader.o3d_parts_bent(
            &o3d_path,
            tex_dirs,
            &overrides,
            bend.map(|(p, b)| (p.as_path(), b.as_ref())),
        ));
    }
    Some(mesh_parts)
}

/// The levels of detail of a parked/static vehicle model in the order they are tried: the most
/// detailed level first. `[LOD] x` is the smallest share of the screen the level is drawn at, so
/// the highest threshold holds the full model and `[LOD] 0` the coarse far one (corpus: the
/// `0.07` + `0` configs keep the body, wheels and shadow decal at `0.07` and a single `*_LOD.o3d`
/// at `0`; Aachen's `FW_*` cars leave `[LOD] 0` without a `[mesh]`). OPTS has no distance switch
/// for these objects yet. Levels without a `[mesh]` are left out.
pub(super) fn lod_preference(lods: &[ModelLod]) -> Vec<&ModelLod> {
    let mut out: Vec<&ModelLod> = lods.iter().filter(|l| !l.meshes.is_empty()).collect();
    out.sort_by(|a, b| b.threshold.total_cmp(&a.threshold));
    out
}
