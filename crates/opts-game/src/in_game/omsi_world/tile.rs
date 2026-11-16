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
mod zones;
pub use zones::sync_tile_zones;

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

mod object;
mod special;
use object::spawn_object_inner;
use special::special_object;

mod finish;
mod meshes;
use meshes::load_sco_meshes;
#[cfg(test)]
use meshes::lod_preference;

#[cfg(test)]
mod tests;
