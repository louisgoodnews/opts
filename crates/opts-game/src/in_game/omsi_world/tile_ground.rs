//! Ground stages of a tile: terrain mesh (carved under the drawn ground) and spline ribbons.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{
    build_terrain_mesh, build_water_mesh, carve_terrain, extrude_spline_segment,
    spline_ground_triangles, terrain_material, water_material, ContentLoader, GroundTriangle,
    TileTerrain,
};
use opts_core::formats::map::{MapTerrain, MapTile, MapWater};

use super::ground::compose_tile_material;
use super::{WorldLoadJob, WorldStats};

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

/// The drawn ground surfaces that can lie over tile `key`'s terrain, in its frame: the surface
/// ribbons of the splines of the tile and its eight neighbours (splines are stored on the tile
/// they start in but run on across the border).
fn carve_surfaces(job: &mut WorldLoadJob, key: (i32, i32)) -> Vec<GroundTriangle> {
    let mut out = Vec::new();
    for (dx, dy) in NEIGHBOURHOOD {
        let other = (key.0 + dx, key.1 + dy);
        let Some(splines) = job.splines_of(other) else {
            continue;
        };
        let o = job.grid.neighbour_offset(key, other);
        for inst in splines.iter() {
            if let Some(profile) = job.sli(&inst.sli_file) {
                out.extend(spline_ground_triangles(&profile, inst, (o.x, o.y)));
            }
        }
    }
    out
}

/// Terrain stage: reads the height grid, carves it under the drawn ground and spawns the ground
/// mesh. Returns the carved grid: the ground every object, lane and vehicle stands on.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_terrain(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    ground_materials: Option<&mut Assets<GroundMaterial>>,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
) -> Option<TileTerrain> {
    // --- Terrain ---------------------------------------------------------------------------
    let terrain_rel = format!("{}.terrain", job.tile_files[&key]);
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

    if let Some(t) = &terrain {
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
        // Ground painting: per-layer alpha masks of this tile (road markings, pavements, paths).
        let tile_material = ground_materials
            .and_then(|gm| compose_tile_material(job, key, images, materials, meshes, gm));
        // Terrain must not cover the drawn ground (`docs/src/architecture/terrain-carving.md`).
        let cell = job.terrain_cell(key.1);
        let surfaces = carve_surfaces(job, key);
        let (carved, carve) = carve_terrain(&TileTerrain::new(t.clone(), cell), &surfaces);
        tracing::debug!(
            tile = ?key,
            triangles = surfaces.len(),
            lowered = carve.lowered,
            guarded = carve.guarded,
            "terrain carved"
        );
        let mesh = build_terrain_mesh(&carved.grid, origin, cell, tile_material.is_some());
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
        return Some(carved);
    }
    tracing::warn!(tile = ?key, "tile has no readable .terrain grid; objects use z as absolute height");
    None
}

/// Water stage: the tile's `.map.water` surface over the terrain it floods.
///
/// Drawn when the tile announces water (`[water]` marker) and the file is there: 93 of 1,906
/// water files in the installation lack the marker, scattered over 17 maps, and are taken as
/// water the editor switched off (`docs/src/formats/map/tiles.md`, Q-M07).
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_water(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    tile: &MapTile,
    terrain: Option<&TileTerrain>,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
) {
    if !tile.has_water || !job.water_enabled {
        return;
    }
    let rel = format!("{}.water", job.tile_files[&key]);
    let Some(path) = job.install.find_in(&job.config.map_dir, &rel) else {
        tracing::warn!(tile = ?key, "tile announces water but has no .water file");
        return;
    };
    job.loading_deps
        .insert(opts_bevy::hot_reload::cache::normalize(&path));
    let water = match std::fs::read(&path).map(|b| MapWater::parse(&b)) {
        Ok(Ok(w)) => w,
        Ok(Err(e)) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "water parse failed: {e}");
            return;
        }
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "water read failed: {e}");
            return;
        }
    };
    let Some(terrain) = terrain else {
        tracing::warn!(tile = ?key, "water without a terrain grid is not drawn");
        return;
    };
    let Some(mesh) = build_water_mesh(&terrain.grid, &water, origin, terrain.cell) else {
        tracing::debug!(tile = ?key, levels = ?water.levels, "water lies below the terrain everywhere");
        return;
    };
    let material = job
        .water_material
        .get_or_insert_with(|| materials.add(water_material()))
        .clone();
    stats.scene_vertices += mesh.count_vertices();
    stats.water_tiles += 1;
    job.tile_water.insert(key, water);
    children.push(
        commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Name::new(format!("Water_{}_{}", key.0, key.1)),
            ))
            .id(),
    );
}

/// Spline stage: extrudes every spline's profile and spawns the ribbons.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_splines(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    tile: &MapTile,
    origin: (f32, f32),
    children: &mut Vec<Entity>,
) {
    // --- Splines ---------------------------------------------------------------------------
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            stats.splines_failed += 1;
            continue;
        };
        if profile.only_editor {
            continue;
        }
        let spline_meshes = extrude_spline_segment(&profile, inst, origin);
        if spline_meshes.is_empty() {
            // Profiles without a graphical part (path-only helpers) have nothing to draw.
            tracing::debug!(sli = %inst.sli_file, "spline has no graphical profile");
            continue;
        }
        let sli_key = job.locate(&inst.sli_file).unwrap_or_default();
        // Spline textures live in the profile's own pack folder (`Splines/<Pack>/texture`).
        let sli_dir = sli_key.parent().map(Path::to_path_buf).unwrap_or_default();
        let tex_dirs = vec![
            sli_dir.join("texture"),
            sli_dir,
            job.config.map_dir.join("texture"),
        ];
        for (tex_idx, mesh) in spline_meshes {
            let mat = if let Some(m) = job.spline_materials.get(&(sli_key.clone(), tex_idx)) {
                m.clone()
            } else {
                let tex = profile.textures.get(tex_idx);
                let handle = tex.and_then(|t| {
                    let mut loader = ContentLoader {
                        install: &job.install,
                        cache: &mut job.cache,
                        meshes,
                        materials,
                        images,
                    };
                    loader.texture(&t.file, &tex_dirs)
                });
                let alpha = tex.is_some_and(|t| t.alpha_mode.is_some());
                let m = materials.add(StandardMaterial {
                    base_color_texture: handle,
                    perceptual_roughness: 0.9,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: if alpha {
                        AlphaMode::Mask(0.5)
                    } else {
                        AlphaMode::Opaque
                    },
                    // Road surfaces share their plane with the terrain deformed to them.
                    depth_bias: 4.0,
                    ..default()
                });
                job.spline_materials
                    .insert((sli_key.clone(), tex_idx), m.clone());
                m
            };
            stats.scene_vertices += mesh.count_vertices();
            children.push(
                commands
                    .spawn((
                        (Mesh3d(meshes.add(mesh)), MeshMaterial3d(mat)),
                        Name::new(format!("Spline_{}_{}", inst.id_code, tex_idx)),
                    ))
                    .id(),
            );
        }
        stats.splines_loaded += 1;
    }
}
