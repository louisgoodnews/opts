//! The unit's scripts, sounds, door points, passengers and meshes.

use super::super::super::super::consist_plan::PlannedPart;
use super::*;

/// The unit's own scripts, sounds, door points and passengers, and its place in the consist.
pub(super) fn attach_unit_extras(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (root, seg): (Entity, Entity),
    st: &mut ChainState,
    part: &PlannedPart,
) {
    let (path, manifest, dir) = (&part.path, &part.manifest, &part.dir);
    // The unit's own sounds (an articulated bus can have its engine in the second unit); they
    // read the head's variables, the trailer shares its scripts.
    let mut engine_unit = None;
    if !manifest.script_share
        && super::super::scripts::attach_scripts(
            job,
            commands,
            seg,
            manifest,
            dir,
            super::super::scripts::script_seed(job.install.root(), path, 0),
            Some(st.chain.len()),
        )
    {
        st.owner = seg;
        engine_unit = Some(seg);
    }
    st.consist.push(opts_bevy::ConsistUnit {
        part: engine_unit,
        head: false,
        cables: super::super::scripts::cables_of(manifest),
        turned: part.turned,
    });
    super::super::doors::attach_unit_door_points(job, commands, root, seg, manifest, dir);
    crate::player_sound::attach_unit(job, commands, seg, Some(st.owner), manifest, dir);
    super::super::passengers::attach_unit(
        job,
        commands,
        root,
        seg,
        (engine_unit.is_some() || manifest.script_share).then_some(st.owner),
        manifest,
        dir,
    );
    st.axle_base += manifest.axles.len();
}

/// Builds the unit's meshes onto `seg`: the mesh and vertex counts.
pub(super) fn attach_meshes(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (meshes, materials, images): MeshAssets,
    (seg, owner): (Entity, Entity),
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    (cfg, model, dir): (&Path, &ModelConfig, &Path),
) -> (usize, usize) {
    let model_dir =
        crate::mesh_dir::model_mesh_dir(&job.install, dir, cfg, model);
    let tex_dirs = vec![
        job.install
            .find_in(dir, "texture")
            .unwrap_or_else(|| dir.join("texture")),
        model_dir.join("texture"),
    ];
    job.cache.texture_replacements = repaint_map(job, model);
    let (count, verts, _, _) = load_segment_meshes_for(
        job,
        commands,
        (&mut *meshes, &mut *materials, &mut *images),
        (seg, owner),
        lod0,
        &model_dir,
        &tex_dirs,
    );
    job.cache.texture_replacements.clear();
    (count, verts)
}
