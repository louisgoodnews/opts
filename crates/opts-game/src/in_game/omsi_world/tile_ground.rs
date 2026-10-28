//! Ground stages of a tile: terrain mesh (carved under the drawn ground) and spline ribbons.

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{
    build_cut_terrain_mesh, build_terrain_mesh, carve_terrain, spline_cut_triangles,
    spline_ground_triangles, terrain_material, ContentLoader, GroundTriangle, TerrainCut,
    TileTerrain,
};
use opts_core::formats::map::MapTerrain;

use super::ground::compose_tile_material;
use super::{WorldLoadJob, WorldStats};

mod water_splines;
pub(super) use water_splines::{spawn_splines, spawn_water};

/// A tile and the eight around it, as key offsets.
const NEIGHBOURHOOD: [(i32, i32); 9] = [
    (0, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// What one tile lays on the terrain, in its own frame.
#[derive(Debug, Default)]
pub(super) struct TileGround {
    /// The drawn ground: its splines' surface ribbons, its crossings' and surface objects' faces.
    surfaces: Vec<GroundTriangle>,
    /// The `[heightprofile]` strips of its terrain-aligned splines
    /// (`docs/src/architecture/terrain-carving.md` §10).
    strips: Vec<GroundTriangle>,
}

/// The drawn ground surfaces (`strips: false`) or the aligned strips (`true`) that can lie over
/// tile `key`'s terrain, in its frame: those of the tile and its eight neighbours (splines are
/// stored on the tile they start in but run on across the border).
fn neighbourhood_ground(
    job: &mut WorldLoadJob,
    key: (i32, i32),
    strips: bool,
) -> Vec<GroundTriangle> {
    let mut out = Vec::new();
    for (dx, dy) in NEIGHBOURHOOD {
        let other = (key.0 + dx, key.1 + dy);
        let Some(own) = tile_surfaces(job, other) else {
            continue;
        };
        // The triangles are kept in their own tile's frame; moving them is an addition.
        let o = job.grid.neighbour_offset(key, other);
        let shift = Vec3::new(o.x, o.y, 0.0);
        let tris = if strips { &own.strips } else { &own.surfaces };
        out.extend(tris.iter().map(|t| t.map(|p| p + shift)));
    }
    out
}

/// The drawn ground surfaces of tile `key` in its own frame: the ribbons of its splines and the
/// ground faces of its crossings and surface objects. Computed once per tile and kept
/// (`WorldLoadJob::tile_surfaces`); `None` when the tile has no readable file.
fn tile_surfaces(job: &mut WorldLoadJob, key: (i32, i32)) -> Option<Arc<TileGround>> {
    if let Some(s) = job.tile_surfaces.get(&key) {
        return Some(s.clone());
    }
    let splines = job.splines_of(key)?;
    let aligned = job.aligned_of(key);
    let _t = opts_logging::stream_trace::span("terrain.surfaces_build");
    let mut out = TileGround::default();
    for inst in splines.iter() {
        if let Some(profile) = job.sli(&inst.sli_file) {
            out.surfaces
                .extend(spline_ground_triangles(&profile, inst, (0.0, 0.0)));
            if aligned.contains(&inst.id_code) {
                out.strips
                    .extend(spline_cut_triangles(&profile, inst, (0.0, 0.0)));
            }
        }
    }
    // Crossings (bent by their base) and surface objects lay down ground too.
    out.surfaces
        .extend(job.crossing_ground_triangles(key, Vec2::ZERO));
    let out = Arc::new(out);
    job.tile_surfaces.insert(key, out.clone());
    Some(out)
}

/// Forgets the kept surfaces of tiles that no resident tile's carve can need any more.
pub(super) fn prune_tile_surfaces(job: &mut WorldLoadJob) {
    let resident = &job.resident;
    job.tile_surfaces.retain(|k, _| {
        NEIGHBOURHOOD
            .iter()
            .any(|(dx, dy)| resident.contains_key(&(k.0 + dx, k.1 + dy)))
    });
}

/// What the first part of the terrain stage reads: the height grid and the tile's ground
/// painting.
pub(super) struct TerrainInput {
    terrain: MapTerrain,
    tile_material: Option<Handle<GroundMaterial>>,
}

/// Terrain stage, part 1: reads the height grid and composes the tile's ground material (the
/// per-layer alpha masks: road markings, pavements, paths). `None` when the tile has no readable
/// grid.
pub(super) fn load_terrain(
    job: &mut WorldLoadJob,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    ground_materials: Option<&mut Assets<GroundMaterial>>,
    key: (i32, i32),
) -> Option<TerrainInput> {
    let terrain_rel = format!("{}.terrain", job.tile_files[&key]);
    let t_read = opts_logging::stream_trace::span("terrain.read");
    let terrain: Option<MapTerrain> = job
        .install
        .find_in(&job.config.map_dir, &terrain_rel)
        .inspect(|p| {
            job.loading_deps
                .insert(opts_bevy::hot_reload::cache::normalize(p));
        })
        .and_then(|p| match std::fs::read(&p).map(|b| MapTerrain::parse(&b)) {
            Ok(Ok(t)) => Some(t),
            Ok(Err(e)) => {
                tracing::error!(at = %opts_bevy::position_tag(), path = %p.display(), "terrain parse failed: {e}");
                None
            }
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), path = %p.display(), "terrain read failed: {e}");
                None
            }
        });
    drop(t_read);
    let terrain = terrain?;
    let _t_mat = opts_logging::stream_trace::span("terrain.material");
    if job.ground_material.is_none() {
        let tex = job
            .manifest
            .ground_textures
            .first()
            .map(|g| g.ground_texture.clone());
        let dirs = vec![
            job.config.map_dir.clone(),
            job.config.map_dir.join("texture"),
        ];
        let handle = tex.and_then(|name| {
            let mut loader = ContentLoader {
                install: &job.install,
                cache: &mut job.cache,
                meshes,
                materials,
                images,
            };
            loader.texture(&name, &dirs)
        });
        job.ground_material = Some(materials.add(terrain_material(handle)));
    }
    let tile_material = ground_materials
        .and_then(|gm| compose_tile_material(job, key, images, materials, meshes, gm));
    Some(TerrainInput {
        terrain,
        tile_material,
    })
}

/// Terrain stage, part 2: makes sure the surfaces of one of the nine tiles that can lie over
/// tile `key`'s terrain are computed (`index` into [`NEIGHBOURHOOD`]), so that a tile with several
/// new neighbours spreads that work over frames.
pub(super) fn ensure_surfaces(job: &mut WorldLoadJob, key: (i32, i32), index: usize) {
    let (dx, dy) = NEIGHBOURHOOD[index];
    tile_surfaces(job, (key.0 + dx, key.1 + dy));
}

/// Number of tiles whose surfaces the carve of one tile needs.
pub(super) const NEIGHBOURHOOD_LEN: usize = NEIGHBOURHOOD.len();

/// Terrain stage, part 3: carves the grid under the drawn ground and spawns the ground mesh
/// (`docs/src/architecture/terrain-carving.md`). Returns the carved grid: the ground every object,
/// lane and vehicle stands on.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_terrain(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    input: Option<TerrainInput>,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
) -> Option<TileTerrain> {
    let Some(TerrainInput {
        terrain: t,
        tile_material,
    }) = input
    else {
        tracing::warn!(tile = ?key, "tile has no readable .terrain grid; objects use z as absolute height");
        return None;
    };
    let cell = job.terrain_cell(key.1);
    let t_surf = opts_logging::stream_trace::span("terrain.surfaces");
    let surfaces = neighbourhood_ground(job, key, false);
    let strips = TerrainCut::new(neighbourhood_ground(job, key, true), surfaces.clone(), cell);
    drop(t_surf);
    let t_carve = opts_logging::stream_trace::span("terrain.carve");
    let (carved, carve) = carve_terrain(&TileTerrain::new(t, cell), &surfaces);
    drop(t_carve);
    tracing::debug!(
        tile = ?key,
        triangles = surfaces.len(),
        lowered = carve.lowered,
        guarded = carve.guarded,
        aligned = strips.len(),
        "terrain carved"
    );
    let t_mesh = opts_logging::stream_trace::span("terrain.mesh");
    let uv_per_tile = tile_material.is_some();
    let mesh = if strips.is_empty() {
        build_terrain_mesh(&carved.grid, origin, cell, uv_per_tile)
    } else {
        build_cut_terrain_mesh(&carved.grid, origin, cell, uv_per_tile, &strips)
    };
    let carved = carved.with_cut(strips);
    drop(t_mesh);
    stats.scene_vertices += mesh.count_vertices();
    let name = Name::new(format!("Terrain_{}_{}", key.0, key.1));
    children.push(match tile_material {
        Some(material) => commands
            .spawn(((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material)), name))
            .id(),
        None => commands
            .spawn((
                (
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(job.ground_material.clone().unwrap_or_default()),
                ),
                name,
            ))
            .id(),
    });
    Some(carved)
}
