//! The last steps of placing one scenery object: registration and the lookups it needs. The
//! spawn of the meshes and the root entity is in [`spawn`].

use super::meshes::CrossingBend;
use super::*;
use opts_core::formats::scenery::SceneryObject;

mod spawn;

pub(super) use spawn::{place_meshes, spawn_cfg_object};

/// Records a placed object for its `[attachObj]` children and registers its collision; returns the
/// crossing deformation base, if any.
pub(super) fn register_placed(
    (job, commands): (&mut WorldLoadJob, &mut Commands),
    (obj, sco, sco_path, model_dir): (
        &MapObjectInstance,
        &std::sync::Arc<SceneryObject>,
        &Path,
        &Path,
    ),
    (transform, key, terrain_h): (&Transform, (i32, i32), f32),
) -> Option<CrossingBend> {
    // `[attachObj]` children of this object are placed relative to it.
    job.placed
        .entry(obj.id_code)
        .or_insert_with(|| (*transform, sco.clone()));

    tracing::debug!(
        sco = %sco_path.display(),
        tile = ?key,
        pos = ?obj.position,
        terrain_h,
        heading = obj.heading_deg,
        pitch = ?obj.pitch_deg,
        roll = ?obj.roll_deg,
        "placing scenery object"
    );
    // `[crossing_heightdeformation]`: meshes and collision boxes bent over the base surface.
    let bend = job.crossing_base(sco_path, sco);
    {
        let _t = opts_logging::stream_trace::span("obj.collision");
        super::super::collision::register_object(
            job,
            commands,
            sco,
            model_dir,
            transform,
            key,
            bend.as_ref(),
        );
    }
    super::super::trace::set_transform(transform);
    bend
}

/// Finds the object's `.sco`; counts and traces the failure when it is missing.
pub(super) fn lookup_sco(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    obj: &MapObjectInstance,
) -> Option<(std::path::PathBuf, std::sync::Arc<SceneryObject>)> {
    let t_sco = opts_logging::stream_trace::span("obj.sco");
    let sco_found = job.sco(&obj.sco_file);
    drop(t_sco);
    if sco_found.is_none() {
        stats.objects_failed += 1;
        super::super::trace::warn(|| {
            format!(
                "scenery object file not found or unreadable: {}",
                obj.sco_file
            )
        });
    }
    sco_found
}

/// The object's folder, its model folder and its texture folders.
pub(super) fn object_dirs(
    job: &WorldLoadJob,
    sco_path: &Path,
) -> (
    std::path::PathBuf,
    std::path::PathBuf,
    Vec<std::path::PathBuf>,
) {
    let sco_dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let model_dir = job
        .install
        .find_in(&sco_dir, "model")
        .unwrap_or_else(|| sco_dir.join("model"));
    let tex_dirs = vec![sco_dir.join("texture"), job.config.map_dir.join("texture")];
    (sco_dir, model_dir, tex_dirs)
}

/// The scripts of the object and its string-variable names (`[texttexture_enh]` sources given by
/// name).
pub(super) fn template_and_names(
    job: &mut WorldLoadJob,
    (sco_path, sco): (&Path, &SceneryObject),
) -> (
    Option<std::sync::Arc<super::super::scenery_objects::ScriptTemplate>>,
    Vec<String>,
) {
    let t_tpl = opts_logging::stream_trace::span("obj.script_template");
    let template = job.script_template(sco_path, sco);
    drop(t_tpl);
    let string_names = template
        .as_ref()
        .map(|t| t.string_names.clone())
        .unwrap_or_default();
    (template, string_names)
}
