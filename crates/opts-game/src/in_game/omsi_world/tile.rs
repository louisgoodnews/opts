//! Tile loading: terrain, splines, attachments and scenery objects, plus streaming around the camera.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::{bevy_to_omsi_pos, omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::text_texture::{resolve_text, TextSpec};
use opts_bevy::{ContentLoader, TextureOverride, TileTerrain};
use opts_core::formats::map::MapObjectInstance;
use opts_core::formats::vehicles::model::{ModelConfig, ModelLod};

use crate::in_game::attachments::expand_attachments;
use crate::in_game::types::InGameEntity;

use super::tile_ground::{spawn_splines, spawn_terrain, spawn_water};
use super::trees::spawn_tree;
/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{LoadAssets, WorldLoadJob, WorldStats};

/// Flat width from which a spline without lanes (paved area, apron) carries vehicles (m).
const SURFACE_MIN_WIDTH_M: f32 = 3.5;

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
    job.tile_splines
        .insert(key, std::sync::Arc::new(tile.splines.clone()));
    for w in &tile.warnings {
        tracing::warn!(tile = ?key, "tile record left out: {w}");
    }
    opts_bevy::set_loading_tile(Some(key));
    opts_bevy::scene_trace::begin_tile(key);
    let mut children: Vec<Entity> = Vec::new();
    let origin = job.tile_offset(key);
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

    let terrain = spawn_terrain(
        job,
        stats,
        commands,
        meshes,
        materials,
        images,
        ground_materials,
        key,
        origin,
        &mut children,
    );
    spawn_water(
        job,
        stats,
        commands,
        meshes,
        materials,
        &tile,
        terrain.as_ref(),
        key,
        origin,
        &mut children,
    );
    let splines_failed_before = stats.splines_failed;
    spawn_splines(
        job,
        stats,
        commands,
        meshes,
        materials,
        images,
        &tile,
        origin,
        &mut children,
    );

    if opts_bevy::scene_trace::enabled() {
        let failed = stats.splines_failed.saturating_sub(splines_failed_before);
        super::trace::tile_content(job, &tile, key, origin, terrain.is_some(), failed);
    }

    // Surfaces the player's vehicle can stand on: drivable splines (road or rail lanes) and terrain.
    {
        let mut roads = Vec::new();
        for inst in &tile.splines {
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
            key,
            origin,
            terrain.clone(),
            roads,
            job.start_tile,
            job.grid,
        );
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
            &job.grid,
            |x, y| terrain.as_ref().map_or(0.0, |t| t.height(x, y)),
        )
    };
    let lanes = super::pedestrians::tile_lanes(job, &tile, &attached, terrain.as_ref(), origin);
    super::pedestrians::register_tile(commands, key, lanes);
    let rail = super::rail::tile_rail_lanes(job, &tile, &attached, terrain.as_ref(), origin);
    super::rail::register_tile(commands, key, rail);
    let bikes = super::rail::tile_bike_lanes(job, &tile, origin);
    super::rail::register_bike_tile(commands, key, bikes);
    let road = super::traffic::tile_segments(job, &tile, terrain.as_ref(), origin);
    super::traffic::register_tile(commands, key, road);
    job.scenery.index_tile(&tile);
    let placements = tile
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| (o, Placement::Tile, "objects", i))
        .chain(
            attached
                .iter()
                .enumerate()
                .map(|(i, (o, z))| (o, Placement::OnSpline(*z), "attachments", i)),
        );
    job.placed.clear();
    for (obj, placement, list, index) in placements {
        spawn_object(
            job,
            stats,
            commands,
            meshes,
            materials,
            images,
            obj,
            placement,
            terrain.as_ref(),
            key,
            origin,
            &mut children,
            &tile,
            super::trace::Site {
                list,
                index,
                marker: None,
            },
        );
    }
    place_attached_objects(
        job,
        stats,
        commands,
        meshes,
        materials,
        images,
        &tile,
        terrain.as_ref(),
        key,
        origin,
        &mut children,
    );

    let tile_root = commands
        .spawn((
            Transform::default(),
            InGameEntity,
            TileRoot(key),
            Name::new(format!("Tile_{}_{}", key.0, key.1)),
        ))
        .id();
    commands.entity(tile_root).add_children(&children);
    job.resident.insert(key, tile_root);
    let deps = std::mem::take(&mut job.loading_deps);
    job.tile_deps.insert(key, deps);
    stats.tiles_loaded = job.resident.len();
    stats.meshes_exact = job.cache.stats.exact_meshes;
    stats.asset_failures = job.cache.stats.failures.len();
    opts_bevy::scene_trace::end_tile(key);
    opts_bevy::set_loading_tile(None);
}

/// Where an object is put.
#[derive(Debug, Clone, Copy)]
enum Placement {
    /// At its `[object]` record on the tile (height relative to the terrain unless `[absheight]`).
    Tile,
    /// Resolved from a spline attachment: absolute height for `[absheight]` objects.
    OnSpline(f32),
    /// On an attachment point of another object (`[attachObj]`): the full world transform.
    OnParent(Transform),
}

/// World transform of an object on attachment point `point` of a parent placed at `parent`.
///
/// The point is given in the parent's OMSI axes (x right, y forward, z up; `attach_rot_*` turned
/// as in `[newanim]` frames: `Rx·Ry·Rz`, right-handed); the child's own heading, pitch and roll
/// follow the `[object]` convention (heading clockwise).
fn attachment_transform(
    parent: &Transform,
    point: &opts_core::formats::scenery::sco::ScoAttachment,
    heading_deg: f32,
    pitch_deg: f32,
    roll_deg: f32,
) -> Transform {
    let r = point.rotation_deg;
    // OMSI x -> Bevy x, OMSI y (forward) -> Bevy -z, OMSI z (up) -> Bevy y.
    let frame = Quat::from_rotation_x(r.x.to_radians())
        * Quat::from_rotation_z(-r.y.to_radians())
        * Quat::from_rotation_y(r.z.to_radians());
    let local = Transform {
        translation: omsi_to_bevy_pos(point.translation),
        rotation: frame * omsi_euler_to_bevy_quat(-heading_deg, pitch_deg, roll_deg),
        scale: Vec3::ONE,
    };
    parent.mul_transform(local)
}

/// Places the tile's `[attachObj]` children on their parents (objects, spline attachments or other
/// children placed before them, all on this tile). Chains are resolved in rounds; a child whose
/// parent never appears or whose parent has no such attachment point is logged at WARN.
#[allow(clippy::too_many_arguments)]
fn place_attached_objects(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    tile: &opts_core::formats::map::MapTile,
    terrain: Option<&TileTerrain>,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
) {
    let mut pending: Vec<usize> = (0..tile.attached_objects.len()).collect();
    loop {
        let before = pending.len();
        let mut next = Vec::new();
        for i in pending {
            let a = &tile.attached_objects[i];
            let Some((parent, parent_sco)) = job.placed.get(&a.parent_id).cloned() else {
                next.push(i);
                continue;
            };
            let Some(point) = parent_sco.attachments.get(a.attachment) else {
                tracing::warn!(sco = %a.sco_file, parent = a.parent_id, point = a.attachment, points = parent_sco.attachments.len(), tile = ?key, "attached object skipped: the parent has no such attachment point");
                stats.objects_failed += 1;
                continue;
            };
            let transform = attachment_transform(
                &parent,
                point,
                a.heading_deg,
                a.pitch_deg.unwrap_or(0.0),
                a.roll_deg.unwrap_or(0.0),
            );
            // The record as an ordinary object at the resolved spot (tile-local position for
            // whatever samples it, e.g. trees and the scene trace).
            let world = bevy_to_omsi_pos(transform.translation);
            let obj = MapObjectInstance {
                flag: a.flag,
                sco_file: a.sco_file.clone(),
                id_code: a.id_code,
                position: Vec3::new(world.x - origin.0, world.y - origin.1, 0.0),
                heading_deg: a.heading_deg,
                pitch_deg: a.pitch_deg,
                roll_deg: a.roll_deg,
                string_vars: a.string_vars.clone(),
            };
            spawn_object(
                job,
                stats,
                commands,
                meshes,
                materials,
                images,
                &obj,
                Placement::OnParent(transform),
                terrain,
                key,
                origin,
                children,
                tile,
                super::trace::Site {
                    list: "attached objects",
                    index: i,
                    marker: None,
                },
            );
        }
        pending = next;
        if pending.is_empty() || pending.len() == before {
            break;
        }
    }
    for i in pending {
        let a = &tile.attached_objects[i];
        tracing::warn!(sco = %a.sco_file, id = a.id_code, parent = a.parent_id, tile = ?key, "attached object skipped: its parent is not placed on this tile");
        stats.objects_failed += 1;
    }
}

/// Places one scenery object (or tree) of a tile; the scene trace sees what came of it.
#[allow(clippy::too_many_arguments)]
fn spawn_object(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    obj: &MapObjectInstance,
    placement: Placement,
    terrain: Option<&TileTerrain>,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
    tile: &opts_core::formats::map::MapTile,
    site: super::trace::Site<'_>,
) {
    if !opts_bevy::scene_trace::enabled() {
        spawn_object_inner(
            job, stats, commands, meshes, materials, images, obj, placement, terrain, key, origin,
            children, tile, site,
        );
        return;
    }
    let started = std::time::Instant::now();
    let before = super::trace::Counters::of(job, stats);
    let first = children.len();
    super::trace::begin_frame();
    spawn_object_inner(
        job, stats, commands, meshes, materials, images, obj, placement, terrain, key, origin,
        children, tile, site,
    );
    let frame = super::trace::end_frame();
    let entity = children.get(first).copied();
    super::trace::object(
        job, stats, before, frame, obj, site, tile, key, origin, entity, started,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_object_inner(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    obj: &MapObjectInstance,
    placement: Placement,
    terrain: Option<&TileTerrain>,
    key: (i32, i32),
    origin: (f32, f32),
    children: &mut Vec<Entity>,
    tile: &opts_core::formats::map::MapTile,
    site: super::trace::Site<'_>,
) {
    let Some((sco_path, sco)) = job.sco(&obj.sco_file) else {
        stats.objects_failed += 1;
        super::trace::warn(|| {
            format!(
                "scenery object file not found or unreadable: {}",
                obj.sco_file
            )
        });
        return;
    };
    if let Some(tree) = &sco.tree {
        super::trace::set_kind(opts_bevy::scene_trace::kind::TREE);
        children.extend(spawn_tree(
            job, stats, commands, meshes, materials, images, tree, obj, origin, terrain,
        ));
        return;
    }
    if sco.carpark_p {
        let Some(parked) = job.parked_vehicle(obj) else {
            return;
        };
        let mut replaced = obj.clone();
        replaced.sco_file = parked;
        let is_marker = job
            .sco(&replaced.sco_file)
            .is_some_and(|(_, s)| s.carpark_p);
        super::trace::skip_record();
        if !is_marker {
            spawn_object(
                job,
                stats,
                commands,
                meshes,
                materials,
                images,
                &replaced,
                placement,
                terrain,
                key,
                origin,
                children,
                tile,
                super::trace::Site {
                    marker: Some(&obj.sco_file),
                    ..site
                },
            );
        }
        return;
    }
    // An editor-only object draws no mesh; its `[maplight]`s are placed below.
    if sco.only_editor && sco.map_lights.is_empty() {
        tracing::debug!(sco = %sco_path.display(), "editor-only object skipped");
        return;
    }
    // Route helpers (`[helparrow]`: arrows over junctions, the next stop's name over the stop) are
    // shown by OMSI only while its route-arrow help is on, along the route being driven
    // (`docs/src/architecture/scenery-benchmark.md`); OPTS has no such help yet, and drawing all of
    // them put stop names and arrows in the air all over the maps.
    if sco.is_help_arrow {
        tracing::debug!(sco = %sco_path.display(), "route helper object not drawn");
        return;
    }
    let sco_dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let model_dir = job
        .install
        .find_in(&sco_dir, "model")
        .unwrap_or_else(|| sco_dir.join("model"));
    let tex_dirs = vec![sco_dir.join("texture"), job.config.map_dir.join("texture")];

    // `[absheight]` objects (crossings, bridges, ...) store an absolute elevation.
    let terrain_h = if sco.has_absolute_height() {
        0.0
    } else {
        terrain
            .as_ref()
            .map_or(0.0, |t| t.height(obj.position.x, obj.position.y))
    };
    let z = match (placement, sco.has_absolute_height()) {
        (Placement::OnSpline(z), true) => z,
        _ => terrain_h + obj.position.z,
    };
    let world = Vec3::new(origin.0 + obj.position.x, origin.1 + obj.position.y, z);
    let transform = match placement {
        Placement::OnParent(t) => t,
        _ => Transform {
            translation: omsi_to_bevy_pos(world),
            // OMSI headings are clockwise-positive (compass-style); the helper is CCW.
            rotation: omsi_euler_to_bevy_quat(
                -obj.heading_deg,
                obj.pitch_deg.unwrap_or(0.0),
                obj.roll_deg.unwrap_or(0.0),
            ),
            scale: Vec3::ONE,
        },
    };
    if sco.only_editor {
        // No mesh is drawn, but the `[maplight]`s still light the map (e.g. the
        // `sodiumlight_4L_break_L_Forum.sco` helpers that light a square from hidden lamps).
        let root = commands.spawn((transform, Name::new("MapLights"))).id();
        opts_bevy::scenery_runtime::spawn_map_lights(commands, root, &sco);
        children.push(root);
        tracing::debug!(sco = %sco_path.display(), "editor-only object: map lights only");
        return;
    }
    // `[attachObj]` children of this object are placed relative to it.
    job.placed
        .entry(obj.id_code)
        .or_insert_with(|| (transform, sco.clone()));

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
    // `[crossing_heightdeformation]`: meshes and collision boxes bent over the base surface.
    let bend = job.crossing_base(&sco_path, &sco);
    super::collision::register_object(
        job,
        commands,
        &sco,
        &model_dir,
        &transform,
        key,
        bend.as_ref(),
    );
    super::trace::set_transform(&transform);
    if let Some(cfg_rel) = &sco.model_cfg {
        super::trace::set_model_cfg(cfg_rel);
        // The colour scheme of this placement (`[CTC]` repaints), fixed per object and tile.
        let seed = crate::new_game_screen::repaints::instance_seed(&[
            obj.id_code,
            u64::from(key.0 as u32),
            u64::from(key.1 as u32),
        ]);
        spawn_vehicle_model(
            job, stats, commands, meshes, materials, images, &sco_path, &sco_dir, cfg_rel,
            transform, seed, children,
        );
        return;
    }
    let protected_before = job.cache.stats.protected_meshes;
    let hidden_before = job.cache.stats.hidden_parts;
    // Scripts and string-variable names (`[texttexture_enh]` sources given by name).
    let template = job.script_template(&sco_path, &sco);
    let string_names = template
        .as_ref()
        .map(|t| t.string_names.clone())
        .unwrap_or_default();
    let mut mesh_parts = Vec::with_capacity(sco.meshes.len());
    for (mesh_index, mesh_def) in sco.meshes.iter().enumerate() {
        // `[onlyeditor]` after a `[mesh]` hides that mesh only (helpers next to a visible one).
        if !sco.mesh_in_game(mesh_index) {
            mesh_parts.push(Vec::new());
            continue;
        }
        let found = job.install.find_in(&model_dir, &mesh_def.file);
        if let Some(p) = &found {
            job.loading_deps
                .insert(opts_bevy::hot_reload::cache::normalize(p));
            super::trace::mesh_used(p);
        }
        let Some(o3d_path) = found else {
            job.cache
                .stats
                .failures
                .push(format!("o3d not found: {}", mesh_def.file));
            tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), mesh = %mesh_def.file, "o3d not found");
            super::trace::warn(|| format!("o3d not found: {}", mesh_def.file));
            return;
        };
        let overrides: Vec<TextureOverride> = mesh_def
            .materials
            .iter()
            .map(|m| TextureOverride {
                slot: m.material_index,
                texture: m.texture.clone(),
                alpha: m.alpha_mode,
                alpha_variable: m.alpha_scale.clone(),
                trans_map: None,
                light_map: super::scenery_objects::light_map_of(m),
                image: m.text_texture.and_then(|i| {
                    let tt = sco.text_textures.get(i)?;
                    let Some(text) = resolve_text(
                        tt.string_index,
                        tt.string_var.as_deref(),
                        &obj.string_vars,
                        &string_names,
                    ) else {
                        tracing::error!(at = %opts_bevy::position_tag(),
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
        mesh_parts.push(loader.o3d_parts_bent(
            &o3d_path,
            &tex_dirs,
            &overrides,
            bend.as_ref().map(|(p, b)| (p.as_path(), b.as_ref())),
        ));
    }
    if mesh_parts.iter().all(Vec::is_empty) {
        if sco.meshes.is_empty() || sco.is_editor_only() {
            tracing::debug!(sco = %sco_path.display(), "object has no mesh (helper/path object)");
            stats.objects_no_geometry += 1;
        } else if job.cache.stats.hidden_parts > hidden_before {
            // Every slot is scaled by a script variable (`[alphascale]`, e.g. a night overlay):
            // nothing to draw, which is not a failure (P-1014, MI-0029).
            tracing::debug!(sco = %sco_path.display(), "object has only variable-hidden slots");
            stats.objects_no_geometry += 1;
        } else if job.cache.stats.protected_meshes > protected_before {
            stats.objects_protected += 1;
            tracing::warn!(sco = %sco_path.display(), tile = ?key, pos = ?obj.position, "object skipped: no mesh has usable geometry");
        } else {
            tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), "object has meshes but none could be loaded");
            stats.objects_failed += 1;
        }
        return;
    }

    let name = sco
        .friendly_name
        .clone()
        .unwrap_or_else(|| obj.sco_file.clone());
    let parent = commands.spawn((transform, Name::new(name))).id();
    children.push(parent);
    stats.scene_vertices += mesh_parts
        .iter()
        .flatten()
        .map(|p| p.vertices)
        .sum::<usize>();
    job.finish_object(
        commands, meshes, materials, parent, &sco, obj, mesh_parts, template,
    );
    stats.objects_loaded += 1;
}

/// The levels of detail of a parked/static vehicle model in the order they are tried: the most
/// detailed level first. `[LOD] x` is the smallest share of the screen the level is drawn at, so
/// the highest threshold holds the full model and `[LOD] 0` the coarse far one (corpus: the
/// `0.07` + `0` configs keep the body, wheels and shadow decal at `0.07` and a single `*_LOD.o3d`
/// at `0`; Aachen's `FW_*` cars leave `[LOD] 0` without a `[mesh]`). OPTS has no distance switch
/// for these objects yet. Levels without a `[mesh]` are left out.
fn lod_preference(lods: &[ModelLod]) -> Vec<&ModelLod> {
    let mut out: Vec<&ModelLod> = lods.iter().filter(|l| !l.meshes.is_empty()).collect();
    out.sort_by(|a, b| b.threshold.total_cmp(&a.threshold));
    out
}

/// Places a parked/static vehicle object: its `[model]` is a vehicle-style model config; the first
/// level of detail of [`lod_preference`] that loads anything is drawn.
#[allow(clippy::too_many_arguments)]
fn spawn_vehicle_model(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    sco_path: &Path,
    sco_dir: &Path,
    cfg_rel: &str,
    transform: Transform,
    repaint_seed: u64,
    children: &mut Vec<Entity>,
) {
    let Some(cfg_path) = job.install.find_in(sco_dir, cfg_rel) else {
        tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), cfg = cfg_rel, "vehicle model config not found");
        super::trace::warn(|| format!("vehicle model config not found: {cfg_rel}"));
        stats.objects_failed += 1;
        return;
    };
    job.loading_deps
        .insert(opts_bevy::hot_reload::cache::normalize(&cfg_path));
    let model = match std::fs::read(&cfg_path)
        .map_err(|e| e.to_string())
        .and_then(|b| ModelConfig::parse_bytes(&b).map_err(|e| e.to_string()))
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), cfg = %cfg_path.display(), "vehicle model config failed: {e}");
            stats.objects_failed += 1;
            return;
        }
    };
    let lods = lod_preference(&model.lods);
    if lods.is_empty() {
        tracing::error!(at = %opts_bevy::position_tag(), cfg = %cfg_path.display(), "vehicle model config has no meshes");
        stats.objects_failed += 1;
        return;
    }
    let model_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        job.install
            .find_in(sco_dir, "texture")
            .unwrap_or_else(|| sco_dir.join("texture")),
        model_dir.join("texture"),
    ];
    let root = commands
        .spawn((transform, Name::new(cfg_rel.to_string())))
        .id();
    job.vehicle_text_defs = model.text_textures.clone();
    let repaints = job
        .model_repaints
        .entry(cfg_path.clone())
        .or_insert_with(|| {
            crate::new_game_screen::repaints::discover_for_model(
                &job.config.root,
                sco_dir,
                &model,
                "",
            )
        });
    let replacements = repaints.replacements(repaints.pick(repaint_seed), &model.ctc_textures);
    job.cache.texture_replacements = replacements.into_iter().collect();
    let (mut count, mut verts) = (0, 0);
    for lod in lods {
        let (c, v, _, _) = super::vehicle::load_segment_meshes(
            job, commands, meshes, materials, images, root, lod, &model_dir, &tex_dirs,
        );
        (count, verts) = (c, v);
        if count > 0 {
            break;
        }
    }
    job.cache.texture_replacements.clear();
    if count == 0 {
        tracing::warn!(sco = %sco_path.display(), cfg = %cfg_path.display(), "vehicle model object skipped: no level of detail could be loaded");
        super::trace::warn(|| format!("no level of detail of {cfg_rel} could be loaded"));
        commands.entity(root).despawn();
        stats.objects_failed += 1;
        return;
    }
    children.push(root);
    stats.scene_vertices += verts;
    stats.objects_loaded += 1;
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
    let Ok(cam) = camera.single() else { return };
    let p = cam.translation();
    let centre = job.grid.key_at(Vec2::new(p.x, -p.z), job.start_tile);
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
            commands.entity(root).despawn();
            super::pedestrians::unregister_tile(&mut commands, key);
            super::ground_height::unregister(&mut commands, key);
            super::rail::unregister_tile(&mut commands, key);
            super::rail::unregister_bike_tile(&mut commands, key);
            super::collision::unregister_tile(&mut commands, key);
            super::traffic::unregister_tile(&mut commands, key);
            opts_bevy::scene_trace::forget_tile(key);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of `Vehicles/FW_AstraG/model/Astra_Parked.cfg`: the body in the close-up level,
    /// `[LOD] 0` holding a bare file name without `[mesh]`.
    const PARKED: &str = "[LOD]\r\n0.025\r\n\r\n[mesh]\r\nbody.o3d\r\n\r\n[mesh]\r\nwheels.o3d\r\n\r\n[LOD]\r\n0\r\n\r\n..\\..\\other\\model\\far.o3d\r\n";

    #[test]
    fn a_parked_model_without_meshes_at_lod_0_draws_its_close_up_level() {
        let model = ModelConfig::parse_bytes(PARKED.as_bytes()).expect("parses");
        let order = lod_preference(&model.lods);
        assert_eq!(order.len(), 1);
        assert!((order[0].threshold - 0.025).abs() < 1e-6);
        assert_eq!(order[0].meshes.len(), 2);
    }

    #[test]
    fn an_attached_object_sits_on_the_turned_parent_and_follows_the_point_rotation() {
        use opts_core::formats::scenery::sco::ScoAttachment;
        // Parent facing east (heading 90, clockwise), point 1 m forward and 2 m up.
        let parent = Transform::from_rotation(omsi_euler_to_bevy_quat(-90.0, 0.0, 0.0));
        let point = ScoAttachment {
            translation: Vec3::new(0.0, 1.0, 2.0),
            rotation_deg: Vec3::ZERO,
        };
        let t = attachment_transform(&parent, &point, 0.0, 0.0, 0.0);
        assert!(
            (t.translation - Vec3::new(1.0, 2.0, 0.0)).length() < 1e-5,
            "{t:?}"
        );
        // The child faces where the parent faces: east (Bevy +x).
        let forward = t.rotation * Vec3::NEG_Z;
        assert!((forward - Vec3::X).length() < 1e-5, "{forward}");

        // `attach_rot_z 90` turns the child counter-clockwise seen from above: north -> west.
        let point = ScoAttachment {
            translation: Vec3::ZERO,
            rotation_deg: Vec3::new(0.0, 0.0, 90.0),
        };
        let t = attachment_transform(&Transform::IDENTITY, &point, 0.0, 0.0, 0.0);
        let forward = t.rotation * Vec3::NEG_Z;
        assert!((forward - Vec3::NEG_X).length() < 1e-5, "{forward}");
        // The child's own heading is clockwise like an [object]: 90 -> east.
        let t = attachment_transform(
            &Transform::IDENTITY,
            &ScoAttachment::default(),
            90.0,
            0.0,
            0.0,
        );
        let forward = t.rotation * Vec3::NEG_Z;
        assert!((forward - Vec3::X).length() < 1e-5, "{forward}");
    }

    /// The shape of `TH_A4_B6_Avant_parked.cfg`: full model at `0.07`, coarser ones below.
    #[test]
    fn the_most_detailed_level_comes_first() {
        let cfg = "[LOD]\r\n0.07\r\n\r\n[mesh]\r\nbody.o3d\r\n\r\n[mesh]\r\nwheels.o3d\r\n\r\n[LOD]\r\n0.04\r\n\r\n[mesh]\r\nbody_lod.o3d\r\n\r\n[LOD]\r\n0\r\n\r\n[mesh]\r\nbody_lod_2.o3d\r\n";
        let model = ModelConfig::parse_bytes(cfg.as_bytes()).expect("parses");
        let order: Vec<f32> = lod_preference(&model.lods)
            .iter()
            .map(|l| l.threshold)
            .collect();
        assert_eq!(order, vec![0.07, 0.04, 0.0]);
    }
}
