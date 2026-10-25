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

mod attached;
pub(super) use attached::place_attached_objects;

/// Height above a road at which an `on_surface` object is drawn (m): enough to avoid z-fighting.
const ON_SURFACE_LIFT_M: f32 = 0.01;

/// Places one scenery object (or tree) of a tile; the scene trace sees what came of it.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_object(
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
    let t_sco = opts_logging::stream_trace::span("obj.sco");
    let sco_found = job.sco(&obj.sco_file);
    drop(t_sco);
    let Some((sco_path, sco)) = sco_found else {
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
        let _t = opts_logging::stream_trace::span("obj.tree");
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
            if matches!(placement, Placement::Tile) {
                super::traffic::register_parked_object(job, commands, key, origin, &replaced);
            }
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
    if sco.only_editor && sco.map_lights.is_empty() && !sco.bus_stop {
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
    let mut z = match (placement, sco.has_absolute_height()) {
        (Placement::OnSpline(z), true) => z,
        _ => terrain_h + obj.position.z,
    };
    // `[rendertype] on_surface` objects (road markings, text on the road) lie on the road they
    // were placed on, not on the terrain below it: the map stores their height above the terrain
    // (0.12 m for the bus text of Ahlheim V5), where the road may be higher or lower.
    // A parked car stands on the road of its parking bay the same way: the terrain under a bay
    // that was built into a street is not carved to the street, so a car placed by terrain
    // height floated above (or sank into) it (Berlin-Spandau, Westerwaldstraße).
    let on_road_plane = sco.render_type.as_deref() == Some("on_surface") || site.marker.is_some();
    if on_road_plane && !sco.has_absolute_height() && matches!(placement, Placement::Tile) {
        let (x, y) = (origin.0 + obj.position.x, origin.1 + obj.position.y);
        if let Some(road) = job
            .tile_ground
            .as_ref()
            .and_then(|g| g.road_height_at(x, y, z))
        {
            // A parked car keeps the height the map gives above its ground; a marking lies a
            // hair over the road.
            z = road
                + if site.marker.is_some() {
                    obj.position.z
                } else {
                    ON_SURFACE_LIFT_M
                };
        }
    }
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
    if sco.only_editor && sco.sound.is_none() {
        // No mesh is drawn, but the `[maplight]`s still light the map (e.g. the
        // `sodiumlight_4L_break_L_Forum.sco` helpers that light a square from hidden lamps).
        let root = commands.spawn((transform, Name::new("MapLights"))).id();
        if sco.bus_stop {
            // The marker of a bus stop (`[busstop]`): where the bus stops and the passengers wait.
            commands.entity(root).insert((
                opts_bevy::bus_stops::BusStopMarker {
                    id_code: obj.id_code,
                },
                Name::new("BusStop"),
            ));
        }
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
    {
        let _t = opts_logging::stream_trace::span("obj.collision");
        super::collision::register_object(
            job,
            commands,
            &sco,
            &model_dir,
            &transform,
            key,
            bend.as_ref(),
        );
    }
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
    let t_tpl = opts_logging::stream_trace::span("obj.script_template");
    let template = job.script_template(&sco_path, &sco);
    drop(t_tpl);
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
                night_map: m.night_map.clone().filter(|t| !t.trim().is_empty()),
                free_texture: None,
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
        if sco.sound.is_some() && (sco.meshes.is_empty() || sco.is_editor_only()) {
            // A sound source without geometry (the `Generic` ambient objects: `[onlyeditor]`
            // mesh, a script that raises the triggers): the root carries the script and the
            // voices.
            let name = sco
                .friendly_name
                .clone()
                .unwrap_or_else(|| obj.sco_file.clone());
            let parent = commands.spawn((transform, Name::new(name))).id();
            children.push(parent);
            job.finish_object(
                commands, meshes, materials, parent, &sco, obj, mesh_parts, template,
            );
            crate::scenery_sound::attach(job, commands, parent, &sco, &sco_dir);
            stats.objects_loaded += 1;
            return;
        }
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

    // Static scenery is baked into per-tile merged meshes (`OPTS_INSTANCING=merge`).
    if opts_bevy::instancing::merge()
        && WorldLoadJob::is_flat_static(&sco, &mesh_parts, template.as_ref())
        && job.scenery.dev_look.is_idle()
        && mesh_parts.iter().flatten().all(|p| {
            opts_bevy::instancing::StaticMerger::can_merge(meshes, materials, &p.mesh, &p.material)
        })
    {
        for p in mesh_parts.iter().flatten() {
            job.cache
                .merger
                .add(meshes, materials, &p.mesh, &p.material, &transform);
            stats.scene_vertices += p.vertices;
        }
        stats.objects_loaded += 1;
        return;
    }
    let name = sco
        .friendly_name
        .clone()
        .unwrap_or_else(|| obj.sco_file.clone());
    let _t_fin = opts_logging::stream_trace::span("obj.entity_spawn");
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
    crate::scenery_sound::attach(job, commands, parent, &sco, &sco_dir);
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
    let model_dir = super::mesh_dir::model_mesh_dir(&job.install, sco_dir, &cfg_path, &model);
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

#[cfg(test)]
mod tests;
