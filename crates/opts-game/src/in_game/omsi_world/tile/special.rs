//! Objects that are not drawn like scenery: trees, parked-car markers, editor-only objects and
//! route helpers.

use super::*;

/// Objects that are not drawn like scenery: trees, parking-bay markers (the vehicle parked there
/// is placed instead), editor-only objects and route helpers. Returns whether the object was
/// handled.
pub(super) fn special_object(
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
    (sco_path, sco): (&Path, &opts_core::formats::scenery::SceneryObject),
) -> bool {
    if let Some(tree) = &sco.tree {
        super::super::trace::set_kind(opts_bevy::scene_trace::kind::TREE);
        let _t = opts_logging::stream_trace::span("obj.tree");
        children.extend(spawn_tree(
            (job, stats, commands),
            (meshes, materials, images),
            (tree, obj, origin, terrain),
        ));
        return true;
    }
    if sco.carpark_p {
        let Some(parked) = job.parked_vehicle(obj) else {
            return true;
        };
        let mut replaced = obj.clone();
        replaced.sco_file = parked;
        let is_marker = job
            .sco(&replaced.sco_file)
            .is_some_and(|(_, s)| s.carpark_p);
        super::super::trace::skip_record();
        if !is_marker {
            if matches!(placement, Placement::Tile) {
                super::super::traffic::register_parked_object(
                    job, commands, key, origin, &replaced,
                );
            }
            spawn_object(
                (job, stats, commands),
                (meshes, materials, images),
                (&replaced, placement, terrain, key, origin),
                (
                    children,
                    tile,
                    super::super::trace::Site {
                        marker: Some(&obj.sco_file),
                        ..site
                    },
                ),
            );
        }
        return true;
    }
    // An editor-only object draws no mesh; its `[maplight]`s are placed below.
    if sco.only_editor && sco.map_lights.is_empty() && !sco.bus_stop {
        tracing::debug!(sco = %sco_path.display(), "editor-only object skipped");
        return true;
    }
    // Route helpers (`[helparrow]`: arrows over junctions, the next stop's name over the stop) are
    // shown by OMSI only while its route-arrow help is on, along the route being driven
    // (`docs/src/architecture/scenery-benchmark.md`); OPTS has no such help yet, and drawing all of
    // them put stop names and arrows in the air all over the maps.
    if sco.is_help_arrow {
        tracing::debug!(sco = %sco_path.display(), "route helper object not drawn");
        return true;
    }
    false
}
