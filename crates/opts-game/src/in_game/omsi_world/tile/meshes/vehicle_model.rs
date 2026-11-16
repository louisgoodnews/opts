//! Parked and static vehicle objects: their model config and the first level of detail that loads.

use std::path::PathBuf;

use super::super::*;
use super::lod_preference;

/// Finds and parses the model config of a vehicle model object (registering it as a hot-reload
/// dependency); counts a failure and returns `None` when it is missing or unreadable.
fn read_model_config(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    (sco_path, sco_dir, cfg_rel): (&Path, &Path, &str),
) -> Option<(PathBuf, ModelConfig)> {
    let Some(cfg_path) = job.install.find_in(sco_dir, cfg_rel) else {
        tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), cfg = cfg_rel, "vehicle model config not found");
        super::super::super::trace::warn(|| format!("vehicle model config not found: {cfg_rel}"));
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
pub(in crate::in_game::omsi_world::tile) fn spawn_vehicle_model(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (sco_path, sco_dir, cfg_rel): (&Path, &Path, &str),
    (transform, repaint_seed): (Transform, u64),
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
        super::super::super::mesh_dir::model_mesh_dir(&job.install, sco_dir, &cfg_path, &model);
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
        let (c, v, _, _) = super::super::super::vehicle::load_segment_meshes(
            job,
            commands,
            (&mut *meshes, &mut *materials, &mut *images),
            root,
            lod,
            &model_dir,
            &tex_dirs,
        );
        (count, verts) = (c, v);
        if count > 0 {
            break;
        }
    }
    job.cache.texture_replacements.clear();
    if count == 0 {
        tracing::warn!(sco = %sco_path.display(), cfg = %cfg_path.display(), "vehicle model object skipped: no level of detail could be loaded");
        super::super::super::trace::warn(|| {
            format!("no level of detail of {cfg_rel} could be loaded")
        });
        commands.entity(root).despawn();
        stats.objects_failed += 1;
        return;
    }
    children.push(root);
    stats.scene_vertices += verts;
    stats.objects_loaded += 1;
}
