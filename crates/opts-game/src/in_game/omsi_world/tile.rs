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

mod attached;
pub(super) use attached::place_attached_objects;

/// Height above a road at which an `on_surface` object is drawn (m): enough to avoid z-fighting.
const ON_SURFACE_LIFT_M: f32 = 0.01;

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

/// Where the object stands: the terrain height at its place (or the absolute one), moved onto the
/// road for markings and parked cars, and the transform from its OMSI heading, pitch and roll.
fn object_transform(
    job: &WorldLoadJob,
    sco: &opts_core::formats::scenery::SceneryObject,
    (obj, placement, terrain, origin): (
        &MapObjectInstance,
        Placement,
        Option<&TileTerrain>,
        (f32, f32),
    ),
    on_marker: bool,
) -> (Transform, f32) {
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
    let on_road_plane = sco.render_type.as_deref() == Some("on_surface") || on_marker;
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
                + if on_marker {
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
    (transform, terrain_h)
}

/// An object that draws no mesh but whose `[maplight]`s still light the map (and a bus stop's
/// marker).
fn spawn_map_lights_only(
    commands: &mut Commands,
    children: &mut Vec<Entity>,
    (obj, sco, sco_path): (
        &MapObjectInstance,
        &opts_core::formats::scenery::SceneryObject,
        &Path,
    ),
    transform: Transform,
) {
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
    opts_bevy::scenery_runtime::spawn_map_lights(commands, root, sco);
    children.push(root);
    tracing::debug!(sco = %sco_path.display(), "editor-only object: map lights only");
}

/// An object whose meshes gave no parts: a sound source without geometry becomes a root with its
/// voices; any other is counted as having no geometry, being protected or having failed.
fn no_mesh_parts(
    (job, stats, commands, meshes, materials): (
        &mut WorldLoadJob,
        &mut WorldStats,
        &mut Commands,
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
    ),
    (obj, sco, sco_path, sco_dir, key): (
        &MapObjectInstance,
        &opts_core::formats::scenery::SceneryObject,
        &Path,
        &Path,
        (i32, i32),
    ),
    (transform, mesh_parts, template, children): ObjParts<'_>,
    (hidden_before, protected_before): (usize, usize),
) {
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
            commands,
            (&mut *meshes, &mut *materials),
            parent,
            (sco, obj),
            mesh_parts,
            template,
        );
        crate::scenery_sound::attach(job, commands, parent, sco, sco_dir);
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
    if special_object(
        (job, stats, commands),
        (meshes, materials, images),
        (obj, placement, terrain, key, origin),
        (children, tile, site),
        (&sco_path, &sco),
    ) {
        return;
    }
    let sco_dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let model_dir = job
        .install
        .find_in(&sco_dir, "model")
        .unwrap_or_else(|| sco_dir.join("model"));
    let tex_dirs = vec![sco_dir.join("texture"), job.config.map_dir.join("texture")];

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
            (job, stats, commands),
            (meshes, materials, images),
            (&sco_path, &sco_dir, cfg_rel),
            (transform, seed),
            children,
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
        commands,
        (&mut *meshes, &mut *materials),
        parent,
        (&sco, obj),
        mesh_parts,
        template,
    );
    crate::scenery_sound::attach(job, commands, parent, &sco, &sco_dir);
    stats.objects_loaded += 1;
}

mod meshes;
#[cfg(test)]
use meshes::lod_preference;
use meshes::{load_sco_meshes, spawn_vehicle_model};

#[cfg(test)]
mod tests;
