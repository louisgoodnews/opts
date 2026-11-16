//! The tile build as a sequence of small steps that can pause between frames (M-0089 step 1).
//!
//! Building a tile used to be one call: on a dense map it kept a frame for 28 ms on average and
//! 153 ms at most while driving, and for seconds on the loading screen
//! (`docs/src/architecture/streaming-performance.md`). The build is now a [`TileBuild`] kept on the
//! job: terrain, splines, surfaces, attachments, registries, then the scenery objects one by one,
//! then the objects attached to other objects. [`advance`] runs steps until the frame's
//! [`FrameBudget`] says stop; the next frame continues. The tile root exists from the first step
//! and the children are added to it after every call, so unloading a half-built tile (the camera
//! moved on, a hot reload) despawns everything the build spawned so far.

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::TileTerrain;
use opts_core::formats::map::{MapObjectInstance, MapTile};

use crate::in_game::stream_budget::FrameBudget;
use crate::in_game::types::InGameEntity;

use super::tile::{place_attached_objects, TileRoot};
use super::tile_ground::{ensure_surfaces, load_terrain, TerrainInput, NEIGHBOURHOOD_LEN};
use super::{WorldLoadJob, WorldStats};

mod begin;
mod steps;
mod terrain;
pub(super) use begin::begin;
use terrain::{terrain_read_step, terrain_surfaces_step};

use steps::{
    attachments_step, finish, objects_step, registries_step, splines_step, surfaces_step,
    terrain_step,
};

/// The step a build is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    /// The terrain grid and the tile's ground painting.
    TerrainRead,
    /// The ground surfaces of the tile and its neighbours, one tile per step.
    TerrainSurfaces,
    /// Carve and mesh the terrain, water.
    Terrain,
    /// Spline meshes.
    Splines,
    /// The surfaces the player's vehicle can stand on.
    Surfaces,
    /// Spline attachments become placements.
    Attachments,
    /// Pedestrian, rail, bike and road registries.
    Registries,
    /// The scenery objects, one per step.
    Objects,
    /// Objects attached to other objects.
    Attached,
    /// Merge, bookkeeping, statistics.
    Finish,
}

/// A tile that is being built over several frames.
pub(super) struct TileBuild {
    /// The tile.
    pub(super) key: (i32, i32),
    pub(super) tile: MapTile,
    pub(super) origin: (f32, f32),
    pub(super) root: Entity,
    /// Children spawned since the last call, not yet added to the root.
    pub(super) children: Vec<Entity>,
    pub(super) terrain: Option<TileTerrain>,
    /// What the terrain read produced, until the terrain is built.
    pub(super) terrain_input: Option<TerrainInput>,
    /// Neighbours whose surfaces are ready (index into the nine-tile neighbourhood).
    pub(super) next_neighbour: usize,
    pub(super) attached: Vec<(MapObjectInstance, f32)>,
    pub(super) splines_failed_before: usize,
    pub(super) phase: Phase,
    pub(super) next_object: usize,
    /// The order in which the objects (tile, then attached) are placed; planned at the first
    /// objects step (M-0114).
    pub(super) order: Option<Vec<usize>>,
}

impl TileBuild {
    /// Fraction of the build that is done (steps and objects), for the loading screen's bar.
    pub(super) fn fraction(&self) -> f32 {
        let objects = (self.tile.objects.len() + self.attached.len()).max(1) as f32;
        match self.phase {
            Phase::TerrainRead | Phase::TerrainSurfaces | Phase::Terrain => 0.0,
            Phase::Splines => 0.08,
            Phase::Surfaces => 0.12,
            Phase::Attachments => 0.14,
            Phase::Registries => 0.16,
            Phase::Objects => 0.2 + 0.7 * (self.next_object as f32 / objects),
            Phase::Attached => 0.9,
            Phase::Finish => 0.95,
        }
    }
}

/// Runs the steps of the build in `job.building` until `budget` says stop or the tile is done.
/// Returns `true` when the tile is finished (or there was nothing to build).
pub(super) fn advance(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    mut ground_materials: Option<&mut Assets<GroundMaterial>>,
    budget: &mut FrameBudget,
) -> bool {
    let Some(mut b) = job.building.take() else {
        return true;
    };
    let _t_total = opts_logging::stream_trace::span("tile.build");
    opts_bevy::set_loading_tile(Some(b.key));
    let finished = loop {
        // A new step starts only while the frame has time left (the first one always does).
        if b.phase != Phase::Finish && !budget.allow() {
            break false;
        }
        match b.phase {
            Phase::TerrainRead => terrain_read_step(
                &mut b,
                job,
                (meshes, materials, images),
                ground_materials.as_deref_mut(),
            ),
            Phase::TerrainSurfaces => terrain_surfaces_step(&mut b, job),
            Phase::Terrain => terrain_step(&mut b, job, stats, commands, meshes, materials),
            Phase::Splines => splines_step(&mut b, job, stats, commands, meshes, materials, images),
            Phase::Surfaces => surfaces_step(&mut b, job, commands),
            Phase::Attachments => attachments_step(&mut b, job),
            Phase::Registries => registries_step(&mut b, job, commands),
            Phase::Objects => {
                if objects_step(
                    &mut b,
                    (job, stats, commands),
                    (meshes, materials, images),
                    budget,
                ) {
                    b.phase = Phase::Attached;
                }
            }
            Phase::Attached => {
                place_attached_objects(
                    (job, stats, commands),
                    (meshes, materials, images),
                    (&b.tile, b.terrain.as_ref(), b.key, b.origin),
                    &mut b.children,
                );
                b.phase = Phase::Finish;
            }
            Phase::Finish => {
                finish(&mut b, job, stats, commands, meshes);
                break true;
            }
        }
    };
    if !b.children.is_empty() {
        commands.entity(b.root).add_children(&b.children);
        b.children.clear();
    }
    opts_bevy::set_loading_tile(None);
    if finished {
        opts_bevy::scene_trace::end_tile(b.key);
    } else {
        job.building = Some(b);
    }
    finished
}
