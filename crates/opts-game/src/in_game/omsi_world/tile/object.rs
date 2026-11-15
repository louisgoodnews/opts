//! Places one scenery object after its kind is known: model parts, placement and children.

use super::*;

pub(super) fn spawn_object_inner(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (obj, placement, terrain, key, origin): ObjSpot<'_>,
    (children, tile, site): (
        &mut Vec<Entity>,
        &opts_core::formats::map::MapTile,
        super::super::trace::Site<'_>,
    ),
) {
    let Some((sco_path, sco)) = finish::lookup_sco(job, stats, obj) else {
        return;
    };
    if special_object(
        (job, stats, commands),
        (meshes, materials, images),
        (obj, placement, terrain, key, origin),
        (children, tile, site),
        (&sco_path, &sco),
    ) {
        return;
    }
    let (sco_dir, model_dir, tex_dirs) = finish::object_dirs(job, &sco_path);
    let (transform, terrain_h) = object_transform(
        job,
        &sco,
        (obj, placement, terrain, origin),
        site.marker.is_some(),
    );
    if sco.only_editor && sco.sound.is_none() {
        spawn_map_lights_only(commands, children, (obj, &sco, &sco_path), transform);
        return;
    }
    let bend = finish::register_placed(
        (job, commands),
        (obj, &sco, &sco_path, &model_dir),
        (&transform, key, terrain_h),
    );
    if let Some(cfg_rel) = &sco.model_cfg {
        finish::spawn_cfg_object(
            (job, stats, commands),
            (meshes, materials, images),
            (obj, key, transform),
            (&sco_path, &sco_dir, cfg_rel),
            children,
        );
        return;
    }
    let protected_before = job.cache.stats.protected_meshes;
    let hidden_before = job.cache.stats.hidden_parts;
    let (template, string_names) = finish::template_and_names(job, (&sco_path, &sco));
    let Some(mesh_parts) = load_sco_meshes(
        job,
        (meshes, materials, images),
        (&sco, &sco_path, &model_dir, &tex_dirs),
        (obj, key, &string_names, bend.as_ref()),
    ) else {
        return;
    };
    if mesh_parts.iter().all(Vec::is_empty) {
        let counts = (hidden_before, protected_before);
        no_mesh_parts(
            (job, stats, commands, meshes, materials),
            (obj, &sco, &sco_path, &sco_dir, key),
            (transform, mesh_parts, template, children),
            counts,
        );
        return;
    }

    finish::place_meshes(
        (job, stats, commands),
        (meshes, materials),
        (obj, &sco, &sco_dir),
        (transform, mesh_parts, template, children),
    );
}
