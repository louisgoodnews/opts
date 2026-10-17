//! Ground stages of a tile: terrain mesh (carved under roads), water surface and spline ribbons.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{
    build_terrain_mesh, carve_terrain_under_roads, extrude_spline_segment, road_footprint,
    terrain_material, ContentLoader, RoadFootprint,
};
use opts_core::formats::map::{MapTerrain, MapTile, MapWater};

use super::ground::compose_tile_material;
use super::{WorldLoadJob, WorldStats};

/// Terrain stage: reads the height grid, carves it under roads and spawns the ground mesh.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_terrain(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    ground_materials: Option<&mut Assets<GroundMaterial>>,
    tile: &MapTile,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
) -> Option<MapTerrain> {
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
        // Terrain under height-profile splines must not cover the road surface.
        let roads: Vec<RoadFootprint> = tile
            .splines
            .iter()
            .filter_map(|inst| {
                let profile = job.sli(&inst.sli_file)?;
                road_footprint(&profile, inst)
            })
            .collect();
        let carved = carve_terrain_under_roads(t, &roads);
        let mesh = build_terrain_mesh(&carved, origin, tile_material.is_some());
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
    } else {
        tracing::warn!(tile = ?key, "tile has no readable .terrain grid; objects use z as absolute height");
    }

    terrain
}

/// Water stage: the tile's `.map.water` becomes one translucent surface patch whose heights
/// follow the four corner levels (`docs/src/architecture/terrain-rendering.md`, section 7).
///
/// Only tiles that have the file get water, and only where the surface lies above the terrain
/// somewhere ([`opts_bevy::water_visible`]); the terrain hides the rest through the depth test.
/// `OPTS_NO_WATER=1` skips water entirely (diagnostics).
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_water(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: Option<&MapTerrain>,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
) {
    if std::env::var_os("OPTS_NO_WATER").is_some() {
        return;
    }
    let water_rel = format!("{}.water", job.tile_files[&key]);
    let Some(path) = job.install.find_in(&job.config.map_dir, &water_rel) else {
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
        tracing::debug!(tile = ?key, "water file without terrain: no surface");
        return;
    };
    if !opts_bevy::water_visible(&water, terrain) {
        tracing::debug!(tile = ?key, levels = ?water.levels, "water level nowhere above the terrain");
        return;
    }
    let mesh = opts_bevy::build_water_mesh(water.levels, origin, opts_bevy::tile_size());
    stats.scene_vertices += mesh.count_vertices();
    let material = job
        .water_material
        .get_or_insert_with(|| materials.add(opts_bevy::water_material()))
        .clone();
    tracing::debug!(tile = ?key, levels = ?water.levels, "water surface");
    stats.water_tiles += 1;
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
