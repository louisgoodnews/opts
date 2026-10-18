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

use crate::in_game::attachments::expand_attachments;
use crate::in_game::stream_budget::FrameBudget;
use crate::in_game::types::InGameEntity;

use super::tile::{place_attached_objects, spawn_object, Placement, TileRoot, SURFACE_MIN_WIDTH_M};
use super::tile_ground::{
    ensure_surfaces, load_terrain, prune_tile_surfaces, spawn_splines, spawn_terrain, spawn_water,
    TerrainInput, NEIGHBOURHOOD_LEN,
};
use super::{WorldLoadJob, WorldStats};

/// The step a build is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
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
    tile: MapTile,
    origin: (f32, f32),
    root: Entity,
    /// Children spawned since the last call, not yet added to the root.
    children: Vec<Entity>,
    terrain: Option<TileTerrain>,
    /// What the terrain read produced, until the terrain is built.
    terrain_input: Option<TerrainInput>,
    /// Neighbours whose surfaces are ready (index into the nine-tile neighbourhood).
    next_neighbour: usize,
    attached: Vec<(MapObjectInstance, f32)>,
    splines_failed_before: usize,
    phase: Phase,
    next_object: usize,
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

/// Starts the next queued tile: reads it, spawns its root and registers it as resident. `None`
/// when the queue holds nothing buildable (a tile that is resident already is skipped, one that
/// cannot be read counts as missing).
pub(super) fn begin(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
) -> Option<TileBuild> {
    let _t_total = opts_logging::stream_trace::span("tile.build");
    loop {
        let key = job.queue.pop_front()?;
        if job.resident.contains_key(&key) {
            continue;
        }
        let t_parse = opts_logging::stream_trace::span("tile.read_parse");
        let Some(tile) = job.read_tile(key) else {
            stats.tiles_missing += 1;
            continue;
        };
        job.tile_splines
            .insert(key, std::sync::Arc::new(tile.splines.clone()));
        // A tile rebuilt in place (Chrono switch) may still hold the collision of its previous
        // build.
        super::collision::unregister_tile(commands, key);
        for w in &tile.warnings {
            tracing::warn!(tile = ?key, "tile record left out: {w}");
        }
        drop(t_parse);
        opts_bevy::scene_trace::begin_tile(key);
        let origin = job.tile_offset(key);
        let tile_path = job.tile_path(key).unwrap_or_default();
        job.loading_deps.clear();
        job.loading_deps
            .insert(opts_bevy::hot_reload::cache::normalize(&tile_path));
        for overlay in job.chrono_overlays(key).to_vec() {
            job.loading_deps
                .insert(opts_bevy::hot_reload::cache::normalize(&overlay));
        }
        tracing::info!(
            tile = ?key,
            path = %tile_path.display(),
            objects = tile.objects.len(),
            splines = tile.splines.len(),
            "tile parsed"
        );
        let root = commands
            .spawn((
                Transform::default(),
                InGameEntity,
                TileRoot(key),
                Name::new(format!("Tile_{}_{}", key.0, key.1)),
            ))
            .id();
        job.resident.insert(key, root);
        return Some(TileBuild {
            key,
            tile,
            origin,
            root,
            children: Vec::new(),
            terrain: None,
            terrain_input: None,
            next_neighbour: 0,
            attached: Vec::new(),
            splines_failed_before: 0,
            phase: Phase::TerrainRead,
            next_object: 0,
        });
    }
}

/// Runs the steps of the build in `job.building` until `budget` says stop or the tile is done.
/// Returns `true` when the tile is finished (or there was nothing to build).
#[allow(clippy::too_many_arguments)]
pub(super) fn advance(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
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
            Phase::TerrainRead => {
                let _t = opts_logging::stream_trace::span("tile.terrain");
                b.terrain_input = load_terrain(
                    job,
                    meshes,
                    materials,
                    images,
                    ground_materials.as_deref_mut(),
                    b.key,
                );
                b.phase = Phase::TerrainSurfaces;
            }
            Phase::TerrainSurfaces => {
                let _t = opts_logging::stream_trace::span("tile.terrain");
                // A tile without a grid needs no surfaces.
                if b.terrain_input.is_none() || b.next_neighbour >= NEIGHBOURHOOD_LEN {
                    b.phase = Phase::Terrain;
                } else {
                    ensure_surfaces(job, b.key, b.next_neighbour);
                    b.next_neighbour += 1;
                }
            }
            Phase::Terrain => terrain_step(&mut b, job, stats, commands, meshes, materials),
            Phase::Splines => splines_step(&mut b, job, stats, commands, meshes, materials, images),
            Phase::Surfaces => surfaces_step(&mut b, job, commands),
            Phase::Attachments => attachments_step(&mut b, job),
            Phase::Registries => registries_step(&mut b, job, commands),
            Phase::Objects => {
                if objects_step(
                    &mut b, job, stats, commands, meshes, materials, images, budget,
                ) {
                    b.phase = Phase::Attached;
                }
            }
            Phase::Attached => {
                place_attached_objects(
                    job,
                    stats,
                    commands,
                    meshes,
                    materials,
                    images,
                    &b.tile,
                    b.terrain.as_ref(),
                    b.key,
                    b.origin,
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

fn terrain_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let _t = opts_logging::stream_trace::span("tile.terrain");
    b.terrain = spawn_terrain(
        job,
        stats,
        commands,
        meshes,
        b.terrain_input.take(),
        b.key,
        b.origin,
        &mut b.children,
    );
    spawn_water(
        job,
        stats,
        commands,
        meshes,
        materials,
        &b.tile,
        b.terrain.as_ref(),
        b.key,
        b.origin,
        &mut b.children,
    );
    b.splines_failed_before = stats.splines_failed;
    b.phase = Phase::Splines;
}

fn splines_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let _t = opts_logging::stream_trace::span("tile.splines");
    spawn_splines(
        job,
        stats,
        commands,
        meshes,
        materials,
        images,
        &b.tile,
        b.origin,
        &mut b.children,
    );
    if opts_bevy::scene_trace::enabled() {
        let failed = stats.splines_failed.saturating_sub(b.splines_failed_before);
        super::trace::tile_content(job, &b.tile, b.key, b.origin, b.terrain.is_some(), failed);
    }
    b.phase = Phase::Surfaces;
}

fn surfaces_step(b: &mut TileBuild, job: &mut WorldLoadJob, commands: &mut Commands) {
    let _t = opts_logging::stream_trace::span("tile.surfaces");
    // Surfaces the player's vehicle can stand on: drivable splines (road or rail lanes) and
    // terrain.
    let mut roads = Vec::new();
    for inst in &b.tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        if !profile.only_editor
            && (profile
                .paths
                .iter()
                .any(|p| p.path_type == 0 || p.path_type == 2)
                || profile.flat_width() >= SURFACE_MIN_WIDTH_M)
            && !profile.graphical_profiles.is_empty()
        {
            roads.push((inst.clone(), (*profile).clone()));
        }
    }
    super::ground_height::register(
        commands,
        b.key,
        b.origin,
        b.terrain.clone(),
        roads,
        job.start_tile,
        job.grid,
    );
    b.phase = Phase::Attachments;
}

fn attachments_step(b: &mut TileBuild, job: &mut WorldLoadJob) {
    let _t = opts_logging::stream_trace::span("tile.attachments");
    // Spline attachments become ordinary placements once resolved against their carrier spline.
    b.attached = if b.tile.spline_attachments.is_empty() {
        Vec::new()
    } else {
        let index = job.spline_index();
        let terrain = b.terrain.as_ref();
        expand_attachments(
            &b.tile.spline_attachments,
            &b.tile.splines,
            &index,
            b.key,
            &job.grid,
            |x, y| terrain.map_or(0.0, |t| t.height(x, y)),
        )
    };
    b.phase = Phase::Registries;
}

fn registries_step(b: &mut TileBuild, job: &mut WorldLoadJob, commands: &mut Commands) {
    let _t = opts_logging::stream_trace::span("tile.registries");
    let (key, origin, terrain) = (b.key, b.origin, b.terrain.as_ref());
    let lanes = super::pedestrians::tile_lanes(job, &b.tile, &b.attached, terrain, origin);
    super::pedestrians::register_tile(commands, key, lanes);
    let rail = super::rail::tile_rail_lanes(job, &b.tile, &b.attached, terrain, origin);
    super::rail::register_tile(commands, key, rail);
    let bikes = super::rail::tile_bike_lanes(job, &b.tile, origin);
    super::rail::register_bike_tile(commands, key, bikes);
    let road = super::traffic::tile_segments(job, &b.tile, terrain, origin);
    super::traffic::register_tile(commands, key, road);
    job.scenery.index_tile(&b.tile);
    b.phase = Phase::Objects;
    b.next_object = 0;
    job.placed.clear();
}

/// Spawns the scenery objects from where the build stopped, each one a step of its own. Returns
/// `true` when all are placed.
#[allow(clippy::too_many_arguments)]
fn objects_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    budget: &mut FrameBudget,
) -> bool {
    let _t = opts_logging::stream_trace::span("tile.objects");
    let on_tile = b.tile.objects.len();
    let total = on_tile + b.attached.len();
    let mut first = true;
    while b.next_object < total {
        // The step that was granted covers the first object; every further one asks again.
        if !first && !budget.allow() {
            return false;
        }
        first = false;
        let i = b.next_object;
        let (obj, placement, list, index) = if i < on_tile {
            (&b.tile.objects[i], Placement::Tile, "objects", i)
        } else {
            let (o, z) = &b.attached[i - on_tile];
            (o, Placement::OnSpline(*z), "attachments", i - on_tile)
        };
        spawn_object(
            job,
            stats,
            commands,
            meshes,
            materials,
            images,
            obj,
            placement,
            b.terrain.as_ref(),
            b.key,
            b.origin,
            &mut b.children,
            &b.tile,
            super::trace::Site {
                list,
                index,
                marker: None,
            },
        );
        b.next_object += 1;
    }
    true
}

fn finish(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
) {
    if opts_bevy::instancing::merge() {
        let merged = job.cache.merger.flush(commands, meshes);
        b.children.extend(merged);
        let st = job.cache.merger.stats;
        tracing::info!(
            tile = ?b.key,
            parts_merged = st.parts_merged,
            entities = st.entities,
            refused = st.refused,
            "static scenery merged (running totals)"
        );
    }
    let _t = opts_logging::stream_trace::span("tile.root_adopt");
    let deps = std::mem::take(&mut job.loading_deps);
    job.tile_deps.insert(b.key, deps);
    prune_tile_surfaces(job);
    stats.tiles_loaded = job.resident.len();
    stats.meshes_exact = job.cache.stats.exact_meshes;
    stats.asset_failures = job.cache.stats.failures.len();
}
