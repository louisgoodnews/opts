//! Loading scenery-object mesh parts and static/parked vehicle model configs on a tile.

use std::path::PathBuf;

use super::*;
use opts_bevy::ModelPart;
use opts_core::formats::scenery::SceneryObject;

pub(super) type CrossingBend = (
    PathBuf,
    std::sync::Arc<opts_core::formats::scenery::BaseSurface>,
);


/// The texture overrides of one `[mesh]` of a scenery object: the textures, alpha and light maps
/// of its materials, and the rendered text of its text textures.
fn texture_overrides(
    job: &mut WorldLoadJob,
    images: &mut Assets<Image>,
    (sco, mesh_def): (&SceneryObject, &opts_core::formats::scenery::ScoMesh),
    (sco_path, obj, key, string_names): (&Path, &MapObjectInstance, (i32, i32), &[String]),
) -> Vec<TextureOverride> {
    mesh_def
        .materials
        .iter()
        .map(|m| TextureOverride {
            slot: m.material_index,
            texture: m.texture.clone(),
            alpha: m.alpha_mode,
            alpha_variable: m.alpha_scale.clone(),
            trans_map: None,
            light_map: super::super::scenery_objects::light_map_of(m),
            night_map: m.night_map.clone().filter(|t| !t.trim().is_empty()),
            free_texture: None,
            image: m.text_texture.and_then(|i| {
                let tt = sco.text_textures.get(i)?;
                let Some(text) = resolve_text(
                    tt.string_index,
                    tt.string_var.as_deref(),
                    &obj.string_vars,
                    string_names,
                ) else {
                    tracing::error!(at = %opts_bevy::position_tag(),
                        sco = %sco_path.display(),
                        source = ?tt.string_var.as_deref().unwrap_or("by number"),
                        index = tt.string_index,
                        "text texture has no string on this object instance"
                    );
                    return None;
                };
                let spec = TextSpec {
                    font: tt.font_name.clone(),
                    text,
                    width: tt.width,
                    height: tt.height,
                    mono: tt.color_mode == 0,
                    rgb: tt.text_color,
                    orientation: tt.orientation,
                };
                tracing::debug!(
                    sco = %sco_path.display(),
                    text = %spec.text,
                    tile = ?key,
                    pos = ?obj.position,
                    "text texture"
                );
                job.fonts.render(&job.install, images, &spec)
            }),
        })
        .collect()
}

/// Loads the mesh parts of a scenery object's `[mesh]` records; `None` when a required `.o3d` file
/// is missing.
#[allow(clippy::too_many_arguments)]
pub(super) fn load_sco_meshes(
    job: &mut WorldLoadJob,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    sco: &SceneryObject,
    sco_path: &Path,
    model_dir: &Path,
    tex_dirs: &[PathBuf],
    obj: &MapObjectInstance,
    key: (i32, i32),
    string_names: &[String],
    bend: Option<&CrossingBend>,
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

/// Finds and parses the model config of a vehicle model object (registering it as a hot-reload
/// dependency); counts a failure and returns `None` when it is missing or unreadable.
fn read_model_config(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    (sco_path, sco_dir, cfg_rel): (&Path, &Path, &str),
) -> Option<(PathBuf, ModelConfig)> {
    let Some(cfg_path) = job.install.find_in(sco_dir, cfg_rel) else {
        tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), cfg = cfg_rel, "vehicle model config not found");
        super::super::trace::warn(|| format!("vehicle model config not found: {cfg_rel}"));
        stats.objects_failed += 1;
        return None;
    };
    job.loading_deps
        .insert(opts_bevy::hot_reload::cache::normalize(&cfg_path));
    let model = match std::fs::read(&cfg_path)
        .map_err(|e| e.to_string())
        .and_then(|b| ModelConfig::parse_bytes(&b).map_err(|e| e.to_string()))
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), cfg = %cfg_path.display(), "vehicle model config failed: {e}");
            stats.objects_failed += 1;
            return None;
        }
    };
    Some((cfg_path, model))
}

/// Places a parked/static vehicle object: its `[model]` is a vehicle-style model config; the first
/// level of detail of [`lod_preference`] that loads anything is drawn.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_vehicle_model(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    sco_path: &Path,
    sco_dir: &Path,
    cfg_rel: &str,
    transform: Transform,
    repaint_seed: u64,
    children: &mut Vec<Entity>,
) {
    let Some((cfg_path, model)) = read_model_config(job, stats, (sco_path, sco_dir, cfg_rel))
    else {
        return;
    };
    let lods = lod_preference(&model.lods);
    if lods.is_empty() {
        tracing::error!(at = %opts_bevy::position_tag(), cfg = %cfg_path.display(), "vehicle model config has no meshes");
        stats.objects_failed += 1;
        return;
    }
    let model_dir =
        super::super::mesh_dir::model_mesh_dir(&job.install, sco_dir, &cfg_path, &model);
    let tex_dirs = vec![
        job.install
            .find_in(sco_dir, "texture")
            .unwrap_or_else(|| sco_dir.join("texture")),
        model_dir.join("texture"),
    ];
    let root = commands
        .spawn((transform, Name::new(cfg_rel.to_string())))
        .id();
    job.vehicle_text_defs = model.text_textures.clone();
    let repaints = job
        .model_repaints
        .entry(cfg_path.clone())
        .or_insert_with(|| {
            crate::new_game_screen::repaints::discover_for_model(
                &job.config.root,
                sco_dir,
                &model,
                "",
            )
        });
    let replacements = repaints.replacements(repaints.pick(repaint_seed), &model.ctc_textures);
    job.cache.texture_replacements = replacements.into_iter().collect();
    let (mut count, mut verts) = (0, 0);
    for lod in lods {
        let (c, v, _, _) = super::super::vehicle::load_segment_meshes(
            job, commands, meshes, materials, images, root, lod, &model_dir, &tex_dirs,
        );
        (count, verts) = (c, v);
        if count > 0 {
            break;
        }
    }
    job.cache.texture_replacements.clear();
    if count == 0 {
        tracing::warn!(sco = %sco_path.display(), cfg = %cfg_path.display(), "vehicle model object skipped: no level of detail could be loaded");
        super::super::trace::warn(|| format!("no level of detail of {cfg_rel} could be loaded"));
        commands.entity(root).despawn();
        stats.objects_failed += 1;
        return;
    }
    children.push(root);
    stats.scene_vertices += verts;
    stats.objects_loaded += 1;
}
