//! Placement helpers of a tile's scenery objects: transform, map-light-only objects and the
//! accounting of objects that gave no mesh parts.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::TileTerrain;
use opts_core::formats::map::MapObjectInstance;

use super::{ObjParts, Placement};
use crate::in_game::omsi_world::{WorldLoadJob, WorldStats};

/// Height above a road at which an `on_surface` object is drawn (m): enough to avoid z-fighting.
const ON_SURFACE_LIFT_M: f32 = 0.01;

/// Where the object stands: the terrain height at its place (or the absolute one), moved onto the
/// road for markings and parked cars, and the transform from its OMSI heading, pitch and roll.
pub(super) fn object_transform(
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
pub(super) fn spawn_map_lights_only(
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
pub(super) fn no_mesh_parts(
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
