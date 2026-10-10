//! Tile loading: terrain, splines, attachments and scenery objects, plus streaming around the camera.

use std::collections::HashSet;
use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::text_texture::{resolve_text, TextSpec};
use opts_bevy::{
    attach_model_parts, build_terrain_mesh, carve_terrain_under_roads, extrude_spline_segment,
    road_footprint, sample_terrain, terrain_material, ContentLoader, RoadFootprint,
    TextureOverride, TILE_SIZE_M,
};
use opts_core::formats::map::MapTerrain;

use crate::in_game::attachments::expand_attachments;
use crate::in_game::types::InGameEntity;

use super::ground::{compose_tile_material, tile_origin};
use super::trees::spawn_tree;
/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{LoadAssets, WorldLoadJob, WorldStats};

/// Loads the next queued tile: terrain, splines and scenery objects.
pub fn load_next_tile(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    ground_materials: Option<&mut Assets<GroundMaterial>>,
) {
    let Some(key) = job.queue.pop_front() else {
        return;
    };
    if job.resident.contains_key(&key) {
        return;
    }
    let Some(tile) = job.read_tile(key) else {
        stats.tiles_missing += 1;
        return;
    };
    let mut children: Vec<Entity> = Vec::new();
    let origin = {
        let (a, b) = (tile_origin(key), tile_origin(job.start_tile));
        (a.0 - b.0, a.1 - b.1)
    };
    let tile_path = job.tile_path(key).unwrap_or_default();
    job.loading_deps.clear();
    job.loading_deps
        .insert(opts_bevy::hot_reload::cache::normalize(&tile_path));
    tracing::info!(
        tile = ?key,
        path = %tile_path.display(),
        objects = tile.objects.len(),
        splines = tile.splines.len(),
        "tile parsed"
    );

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
                tracing::error!(path = %p.display(), "terrain parse failed: {e}");
                None
            }
            Err(e) => {
                tracing::error!(path = %p.display(), "terrain read failed: {e}");
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
                .spawn((
                    MaterialMeshBundle {
                        mesh: meshes.add(mesh),
                        material,
                        ..default()
                    },
                    name,
                ))
                .id(),
            None => commands
                .spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh),
                        material: job.ground_material.clone().unwrap_or_default(),
                        ..default()
                    },
                    name,
                ))
                .id(),
        });
    } else {
        tracing::warn!(tile = ?key, "tile has no readable .terrain grid; objects use z as absolute height");
    }

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
        let sli_key = job.install.find_file(&inst.sli_file).unwrap_or_default();
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
                        PbrBundle {
                            mesh: meshes.add(mesh),
                            material: mat,
                            ..default()
                        },
                        Name::new(format!("Spline_{}_{}", inst.id_code, tex_idx)),
                    ))
                    .id(),
            );
        }
        stats.splines_loaded += 1;
    }

    // --- Scenery objects -------------------------------------------------------------------
    // Spline attachments become ordinary placements once resolved against their carrier spline.
    let attached = if tile.spline_attachments.is_empty() {
        Vec::new()
    } else {
        let index = job.spline_index();
        expand_attachments(
            &tile.spline_attachments,
            &tile.splines,
            &index,
            key,
            |x, y| terrain.as_ref().map_or(0.0, |t| sample_terrain(t, x, y)),
        )
    };
    let placements = tile
        .objects
        .iter()
        .map(|o| (o, None))
        .chain(attached.iter().map(|(o, z)| (o, Some(*z))));
    for (obj, attached_z) in placements {
        let Some((sco_path, sco)) = job.sco(&obj.sco_file) else {
            stats.objects_failed += 1;
            continue;
        };
        if let Some(tree) = &sco.tree {
            children.extend(spawn_tree(
                job,
                stats,
                commands,
                meshes,
                materials,
                images,
                tree,
                obj,
                origin,
                terrain.as_ref(),
            ));
            continue;
        }
        if sco.only_editor {
            tracing::debug!(sco = %sco_path.display(), "editor-only object skipped");
            continue;
        }
        let sco_dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
        let model_dir = job
            .install
            .find_in(&sco_dir, "model")
            .unwrap_or_else(|| sco_dir.join("model"));
        let tex_dirs = vec![sco_dir.join("texture"), job.config.map_dir.join("texture")];

        // `[absheight]` objects (crossings, bridges, ...) store an absolute elevation.
        let terrain_h = if sco.abs_height {
            0.0
        } else {
            terrain
                .as_ref()
                .map_or(0.0, |t| sample_terrain(t, obj.position.x, obj.position.y))
        };
        let z = match (attached_z, sco.abs_height) {
            (Some(z), true) => z,
            _ => terrain_h + obj.position.z,
        };
        let world = Vec3::new(origin.0 + obj.position.x, origin.1 + obj.position.y, z);
        let transform = Transform {
            translation: omsi_to_bevy_pos(world),
            // OMSI headings are clockwise-positive (compass-style); the helper is CCW.
            rotation: omsi_euler_to_bevy_quat(
                -obj.heading_deg,
                obj.pitch_deg.unwrap_or(0.0),
                obj.roll_deg.unwrap_or(0.0),
            ),
            scale: Vec3::ONE,
        };

        tracing::debug!(
            sco = %sco_path.display(),
            tile = ?key,
            pos = ?obj.position,
            terrain_h,
            heading = obj.heading_deg,
            pitch = ?obj.pitch_deg,
            roll = ?obj.roll_deg,
            "placing scenery object"
        );
        let protected_before = job.cache.stats.protected_meshes;
        let deferred_before = job.cache.stats.deferred_meshes;
        let mut parts = Vec::new();
        for mesh_def in &sco.meshes {
            let found = job.install.find_in(&model_dir, &mesh_def.file);
            if let Some(p) = &found {
                job.loading_deps
                    .insert(opts_bevy::hot_reload::cache::normalize(p));
            }
            let Some(o3d_path) = found else {
                job.cache
                    .stats
                    .failures
                    .push(format!("o3d not found: {}", mesh_def.file));
                tracing::error!(sco = %sco_path.display(), mesh = %mesh_def.file, "o3d not found");
                continue;
            };
            let overrides: Vec<TextureOverride> = mesh_def
                .materials
                .iter()
                .map(|m| TextureOverride {
                    slot: m.material_index,
                    texture: m.texture.clone(),
                    alpha: m.alpha_mode,
                    hidden_by_variable: m.alpha_scale.is_some(),
                    trans_map: None,
                    image: m.text_texture.and_then(|i| {
                        let tt = sco.text_textures.get(i)?;
                        let Some(text) = resolve_text(
                            tt.string_index,
                            tt.string_var.as_deref(),
                            &obj.string_vars,
                            &[],
                        ) else {
                            tracing::error!(
                                sco = %sco_path.display(),
                                source = ?tt.string_var.as_deref().unwrap_or("by number"),
                                index = tt.string_index,
                                "text texture has no string on this object instance"
                            );
                            return None;
                        };
                        let spec = TextSpec {
                            font: tt.font_name.clone(),
                            text,
                            width: tt.width,
                            height: tt.height,
                            mono: tt.color_mode == 0,
                            rgb: tt.text_color,
                            orientation: tt.orientation,
                        };
                        tracing::debug!(
                            sco = %sco_path.display(),
                            text = %spec.text,
                            tile = ?key,
                            pos = ?obj.position,
                            "text texture"
                        );
                        job.fonts.render(&job.install, images, &spec)
                    }),
                })
                .collect();
            let mut loader = ContentLoader {
                install: &job.install,
                cache: &mut job.cache,
                meshes,
                materials,
                images,
            };
            parts.extend(loader.o3d_parts(&o3d_path, &tex_dirs, &overrides));
        }
        if parts.is_empty() {
            if sco.meshes.is_empty() {
                tracing::debug!(sco = %sco_path.display(), "object has no mesh (helper/path object)");
                stats.objects_no_geometry += 1;
            } else if job.cache.stats.deferred_meshes > deferred_before {
                tracing::debug!(sco = %sco_path.display(), tile = ?key, "object waits for background mesh decoding");
            } else if job.cache.stats.protected_meshes > protected_before {
                stats.objects_protected += 1;
                tracing::warn!(sco = %sco_path.display(), tile = ?key, pos = ?obj.position, "object skipped: all meshes are scrambled");
            } else {
                tracing::error!(sco = %sco_path.display(), "object has meshes but none could be loaded");
                stats.objects_failed += 1;
            }
            continue;
        }

        let name = sco
            .friendly_name
            .clone()
            .unwrap_or_else(|| obj.sco_file.clone());
        let parent = commands
            .spawn((SpatialBundle::from_transform(transform), Name::new(name)))
            .id();
        children.push(parent);
        stats.scene_vertices += parts.iter().map(|p| p.vertices).sum::<usize>();
        attach_model_parts(commands, parent, &parts, "Mesh");
        stats.objects_loaded += 1;
    }

    let tile_root = commands
        .spawn((
            SpatialBundle::default(),
            InGameEntity,
            TileRoot(key),
            Name::new(format!("Tile_{}_{}", key.0, key.1)),
        ))
        .id();
    commands.entity(tile_root).push_children(&children);
    job.resident.insert(key, tile_root);
    let deps = std::mem::take(&mut job.loading_deps);
    job.tile_deps.insert(key, deps);
    let waiting: HashSet<_> = job
        .cache
        .take_deferred()
        .into_iter()
        .filter(|k| {
            job.cache
                .decode_pool
                .as_ref()
                .is_some_and(|p| p.is_pending(*k))
        })
        .collect();
    if !waiting.is_empty() {
        tracing::info!(tile = ?key, meshes = waiting.len(), "tile rebuilds when its meshes are decoded");
        job.waiting_decode.insert(key, waiting);
    }
    stats.tiles_loaded = job.resident.len();
    stats.asset_failures = job.cache.stats.failures.len();
}

/// Marks the root entity of a streamed tile.
#[derive(Component, Debug, Clone, Copy)]
pub struct TileRoot(pub (i32, i32));

/// Keeps the tiles around the camera loaded and unloads distant ones (one tile load per frame).
///
/// The observer is the 3D camera; tiles within `stream_radius` (Chebyshev distance) are
/// resident, tiles beyond `stream_radius + 1` are despawned (hysteresis avoids thrashing at
/// borders). Parsed objects and textures stay cached, so revisiting a tile is cheap.
#[allow(clippy::too_many_arguments)]
pub fn stream_world_tiles(
    job: Option<ResMut<WorldLoadJob>>,
    state: Res<crate::in_game::types::SimulationLoadState>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut stats: ResMut<WorldStats>,
    mut commands: Commands,
    mut assets: LoadAssets,
) {
    let Some(mut job) = job else { return };
    if !state.is_complete {
        return;
    }
    job.rebuild_decoded_tiles(&mut commands, &mut stats);
    let Ok(cam) = camera.get_single() else { return };
    let p = cam.translation();
    let centre = (
        job.start_tile.0 + (p.x / TILE_SIZE_M).floor() as i32,
        job.start_tile.1 + ((-p.z) / TILE_SIZE_M).floor() as i32,
    );
    let dist = |k: (i32, i32)| (k.0 - centre.0).abs().max((k.1 - centre.1).abs());

    let radius = job.stream_radius;
    let far: Vec<(i32, i32)> = job
        .resident
        .keys()
        .copied()
        .filter(|k| dist(*k) > radius + 1)
        .collect();
    for key in far {
        if let Some(root) = job.resident.remove(&key) {
            commands.entity(root).despawn_recursive();
            tracing::debug!(tile = ?key, "tile unloaded");
        }
    }
    stats.tiles_loaded = job.resident.len();

    if job.queue.is_empty() {
        let mut wanted: Vec<(i32, i32)> = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let k = (centre.0 + dx, centre.1 + dy);
                if !job.resident.contains_key(&k) && job.tile_path(k).is_some() {
                    wanted.push(k);
                }
            }
        }
        wanted.sort_by_key(|k| dist(*k));
        if let Some(k) = wanted.first() {
            job.queue.push_back(*k);
        }
    }
    if !job.queue.is_empty() {
        let (meshes, materials, images) = (
            &mut assets.meshes,
            &mut assets.materials,
            &mut assets.images,
        );
        load_next_tile(
            &mut job,
            &mut stats,
            &mut commands,
            meshes,
            materials,
            images,
            assets.ground_materials.as_deref_mut(),
        );
    }
}

impl WorldLoadJob {
    /// Re-queues tiles whose background mesh decoding finished and refreshes the counters.
    fn rebuild_decoded_tiles(&mut self, commands: &mut Commands, stats: &mut WorldStats) {
        let Some(pool) = self.cache.decode_pool.clone() else {
            return;
        };
        let done = pool.drain_finished();
        if !done.is_empty() {
            for waiting in self.waiting_decode.values_mut() {
                for k in &done {
                    waiting.remove(k);
                }
            }
            let ready: Vec<_> = self
                .waiting_decode
                .iter()
                .filter(|(_, w)| w.is_empty())
                .map(|(k, _)| *k)
                .collect();
            for key in ready {
                self.waiting_decode.remove(&key);
                if let Some(root) = self.resident.remove(&key) {
                    commands.entity(root).despawn_recursive();
                    self.tile_deps.remove(&key);
                    if !self.queue.contains(&key) {
                        self.queue.push_front(key);
                    }
                    tracing::info!(tile = ?key, "tile rebuilt with decoded meshes");
                }
            }
        }
        stats.meshes_decoding = pool.pending_count();
        stats.meshes_decoded = self.cache.stats.decoded_meshes;
        stats.meshes_twin = self.cache.stats.twin_meshes;
    }
}
