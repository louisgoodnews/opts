//! Spawning the meshes of a placed object: merged into the tile when static, else under an entity
//! of their own, and the vehicle models (`[model_cfg]`).

use super::super::*;

use super::super::meshes::spawn_vehicle_model;
use opts_core::formats::scenery::SceneryObject;

/// Bakes static scenery into the per-tile merged meshes (`OPTS_INSTANCING=merge`); returns whether
/// the object was merged.
fn merge_static(
    (job, stats): (&mut WorldLoadJob, &mut WorldStats),
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    (sco, mesh_parts, template): (
        &SceneryObject,
        &[Vec<opts_bevy::ModelPart>],
        Option<&std::sync::Arc<super::super::super::scenery_objects::ScriptTemplate>>,
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

/// Places an object that is a vehicle model (`[model_cfg]`) instead of a mesh list.
pub(in super::super) fn spawn_cfg_object(
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
    super::super::super::trace::set_model_cfg(cfg_rel);
    // The colour scheme of this placement (`[CTC]` repaints), fixed per object and tile.
    let seed = crate::new_game_screen::repaints::instance_seed(&[
        obj.id_code,
        u64::from(key.0 as u32),
        u64::from(key.1 as u32),
    ]);
    spawn_vehicle_model(
        (job, stats, commands),
        (meshes, materials, images),
        (sco_path, sco_dir, cfg_rel),
        (transform, seed),
        children,
    );
}

/// Puts the loaded meshes into the merged tile meshes when static, else under an entity of their own.
pub(in super::super) fn place_meshes(
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
