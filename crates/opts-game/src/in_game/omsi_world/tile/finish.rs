//! The last steps of placing one scenery object: registration, static merging, entity spawn.

use super::meshes::CrossingBend;
use super::*;
use opts_core::formats::scenery::SceneryObject;

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

/// Bakes static scenery into the per-tile merged meshes (`OPTS_INSTANCING=merge`); returns whether
/// the object was merged.
fn merge_static(
    (job, stats): (&mut WorldLoadJob, &mut WorldStats),
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    (sco, mesh_parts, template): (
        &SceneryObject,
        &[Vec<opts_bevy::ModelPart>],
        Option<&std::sync::Arc<super::super::scenery_objects::ScriptTemplate>>,
    ),
    transform: &Transform,
) -> bool {
    if !(opts_bevy::instancing::merge()
        && WorldLoadJob::is_flat_static(sco, mesh_parts, template)
        && job.scenery.dev_look.is_idle()
        && mesh_parts.iter().flatten().all(|p| {
            opts_bevy::instancing::StaticMerger::can_merge(meshes, materials, &p.mesh, &p.material)
        }))
    {
        return false;
    }
    for p in mesh_parts.iter().flatten() {
        job.cache
            .merger
            .add(meshes, materials, &p.mesh, &p.material, transform);
        stats.scene_vertices += p.vertices;
    }
    stats.objects_loaded += 1;
    true
}

/// Spawns the object's root entity with its parts, script and sound.
fn spawn_root(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    (obj, sco, sco_dir): (&MapObjectInstance, &SceneryObject, &Path),
    (transform, mesh_parts, template, children): ObjParts<'_>,
) {
    let name = sco
        .friendly_name
        .clone()
        .unwrap_or_else(|| obj.sco_file.clone());
    let _t_fin = opts_logging::stream_trace::span("obj.entity_spawn");
    let parent = commands.spawn((transform, Name::new(name))).id();
    children.push(parent);
    stats.scene_vertices += mesh_parts
        .iter()
        .flatten()
        .map(|p| p.vertices)
        .sum::<usize>();
    job.finish_object(
        commands,
        (&mut *meshes, &mut *materials),
        parent,
        (sco, obj),
        mesh_parts,
        template,
    );
    crate::scenery_sound::attach(job, commands, parent, sco, sco_dir);
    stats.objects_loaded += 1;
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

/// Places an object that is a vehicle model (`[model_cfg]`) instead of a mesh list.
pub(super) fn spawn_cfg_object(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (obj, key, transform): (&MapObjectInstance, (i32, i32), Transform),
    (sco_path, sco_dir, cfg_rel): (&Path, &Path, &str),
    children: &mut Vec<Entity>,
) {
    super::super::trace::set_model_cfg(cfg_rel);
    // The colour scheme of this placement (`[CTC]` repaints), fixed per object and tile.
    let seed = crate::new_game_screen::repaints::instance_seed(&[
        obj.id_code,
        u64::from(key.0 as u32),
        u64::from(key.1 as u32),
    ]);
    super::meshes::spawn_vehicle_model(
        (job, stats, commands),
        (meshes, materials, images),
        (sco_path, sco_dir, cfg_rel),
        (transform, seed),
        children,
    );
}

/// Puts the loaded meshes into the merged tile meshes when static, else under an entity of their own.
pub(super) fn place_meshes(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    (obj, sco, sco_dir): (&MapObjectInstance, &SceneryObject, &Path),
    (transform, mesh_parts, template, children): ObjParts<'_>,
) {
    let statics = (sco, &mesh_parts[..], template.as_ref());
    if merge_static((job, stats), (meshes, materials), statics, &transform) {
        return;
    }
    spawn_root(
        (job, stats, commands),
        (meshes, materials),
        (obj, sco, sco_dir),
        (transform, mesh_parts, template, children),
    );
}
