//! Tile loading: terrain, splines, attachments and scenery objects, plus streaming around the camera.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::{bevy_to_omsi_pos, omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::text_texture::{resolve_text, TextSpec};
use opts_bevy::{ContentLoader, TextureOverride, TileTerrain};
use opts_core::formats::map::MapObjectInstance;
use opts_core::formats::vehicles::model::{ModelConfig, ModelLod};

use crate::in_game::stream_budget::FrameBudget;
use crate::in_game::tile_loading;

use super::tile_build;
use super::trees::spawn_tree;
/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{LoadAssets, WorldLoadJob, WorldStats};

/// An object instance with where it stands: instance, placement, terrain, tile key and origin.
type ObjSpot<'a> = (
    &'a MapObjectInstance,
    Placement,
    Option<&'a TileTerrain>,
    (i32, i32),
    (f32, f32),
);
/// A placed object's model parts and script template, and the children list they go into.
type ObjParts<'a> = (
    Transform,
    Vec<Vec<opts_bevy::ModelPart>>,
    Option<std::sync::Arc<super::scenery_objects::ScriptTemplate>>,
    &'a mut Vec<Entity>,
);

/// Flat width from which a spline without lanes (paved area, apron) carries vehicles (m).
pub(super) const SURFACE_MIN_WIDTH_M: f32 = 3.5;

/// Tiles queued at once by the streamer (the nearest wanted ones).
const REFILL: usize = 4;

mod streaming;
pub use streaming::{advance_tile_loading, load_next_tile, stream_world_tiles, TileRoot};

/// Where an object is put.
#[derive(Debug, Clone, Copy)]
pub(super) enum Placement {
    /// At its `[object]` record on the tile (height relative to the terrain unless `[absheight]`).
    Tile,
    /// Resolved from a spline attachment: absolute height for `[absheight]` objects.
    OnSpline(f32),
    /// On an attachment point of another object (`[attachObj]`): the full world transform.
    OnParent(Transform),
}

mod placement;
use placement::{no_mesh_parts, object_transform, spawn_map_lights_only};
mod attached;
pub(super) use attached::place_attached_objects;

/// Places one scenery object (or tree) of a tile; the scene trace sees what came of it.
pub(super) fn spawn_object(
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
        super::trace::Site<'_>,
    ),
) {
    if !opts_bevy::scene_trace::enabled() {
        spawn_object_inner(
            (job, stats, commands),
            (meshes, materials, images),
            (obj, placement, terrain, key, origin),
            (children, tile, site),
        );
        return;
    }
    let started = std::time::Instant::now();
    let before = super::trace::Counters::of(job, stats);
    let first = children.len();
    super::trace::begin_frame();
    spawn_object_inner(
        (job, stats, commands),
        (meshes, materials, images),
        (obj, placement, terrain, key, origin),
        (children, tile, site),
    );
    let frame = super::trace::end_frame();
    let entity = children.get(first).copied();
    super::trace::object(
        job,
        stats,
        (before, started),
        frame,
        (obj, site),
        (tile, key, origin),
        entity,
    );
}

/// Objects that are not drawn like scenery: trees, parking-bay markers (the vehicle parked there
/// is placed instead), editor-only objects and route helpers. Returns whether the object was
/// handled.
fn special_object(
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
        super::trace::Site<'_>,
    ),
    (sco_path, sco): (&Path, &opts_core::formats::scenery::SceneryObject),
) -> bool {
    if let Some(tree) = &sco.tree {
        super::trace::set_kind(opts_bevy::scene_trace::kind::TREE);
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
        super::trace::skip_record();
        if !is_marker {
            if matches!(placement, Placement::Tile) {
                super::traffic::register_parked_object(job, commands, key, origin, &replaced);
            }
            spawn_object(
                (job, stats, commands),
                (meshes, materials, images),
                (&replaced, placement, terrain, key, origin),
                (
                    children,
                    tile,
                    super::trace::Site {
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

fn spawn_object_inner(
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
        super::trace::Site<'_>,
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

mod finish;
mod meshes;
use meshes::load_sco_meshes;
#[cfg(test)]
use meshes::lod_preference;

#[cfg(test)]
mod tests;
