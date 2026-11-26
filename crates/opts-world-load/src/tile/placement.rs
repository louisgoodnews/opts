//! Placement helpers of a tile's scenery objects: transform, map-light-only objects and the
//! accounting of objects that gave no mesh parts.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::TileTerrain;
use opts_core::formats::map::MapObjectInstance;

use super::{ObjParts, Placement};
use crate::{WorldLoadJob, WorldStats};

/// Height above a road at which an `on_surface` object is drawn (m): enough to avoid z-fighting.
const ON_SURFACE_LIFT_M: f32 = 0.01;

/// Whether a `.sco` is a level-crossing barrier or one of its arms (`Hs64_Schranke_R.sco`,
/// `Schranke`, `Barriere`); the orientation of these is logged (ISS-059, ISS-060).
fn is_barrier(sco_file: &str) -> bool {
    let lower = sco_file.to_ascii_lowercase();
    lower.contains("schranke") || lower.contains("barriere")
}

/// Objects whose `[object]` heading the map data gets wrong, keyed by map folder and ID code, with
/// the turn to add in OMSI degrees (clockwise). Falkensee Bahnhof (ISS-060): the two arms of the
/// level crossing `bue_schranke1_Main.sco` are placed at headings 107 and 287, across the
/// pedestrian way; the owner's decision is a turn of about 90 degrees.
const HEADING_OVERRIDES: &[(&str, u64, f32)] = &[
    ("Berlin-Spandau", 128680, -90.0),
    ("Berlin-Spandau", 128681, -90.0),
];

/// The heading an object is drawn with: its `[object]` heading, turned by the override of its map
/// and ID code where the map data places it wrongly (ISS-060). OMSI degrees, clockwise.
pub(super) fn heading_with_override(map: &str, id_code: u64, heading_deg: f32) -> f32 {
    HEADING_OVERRIDES
        .iter()
        .find(|(m, id, _)| *m == map && *id == id_code)
        .map_or(heading_deg, |(_, _, turn)| heading_deg + turn)
}

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
    // The map folder names the map (`maps/Berlin-Spandau`) for the per-object overrides.
    let map = job
        .config
        .map_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let heading_deg = heading_with_override(map, obj.id_code, obj.heading_deg);
    if is_barrier(&obj.sco_file) {
        // The angle the barrier is turned by, and where it comes from (ISS-060).
        let source = match placement {
            Placement::OnParent(_) => "parent transform",
            _ if heading_deg != obj.heading_deg => "map [object] heading with a map override",
            _ => "map [object] heading, negated for the CCW helper",
        };
        tracing::debug!(object = %obj.id_code, sco = %obj.sco_file, map, heading = obj.heading_deg, drawn = heading_deg, yaw = -heading_deg, pitch = ?obj.pitch_deg, roll = ?obj.roll_deg, source, "barrier orientation");
    }
    let transform = match placement {
        Placement::OnParent(t) => t,
        _ => Transform {
            translation: omsi_to_bevy_pos(world),
            // OMSI headings are clockwise-positive (compass-style); the helper is CCW.
            rotation: omsi_euler_to_bevy_quat(
                -heading_deg,
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

mod no_mesh;
pub(super) use no_mesh::no_mesh_parts;
