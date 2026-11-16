//! Track and cycle-path lanes of a tile in the session frame and their registration with the networks.

use super::*;

/// Track lanes of a tile in the session frame: splines (rail `[path]` of their profile) and the
/// rail paths of placed objects (points, crossings, junction plates).
pub(in crate::in_game::omsi_world) fn tile_rail_lanes(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    attached: &[(MapObjectInstance, f32)],
    terrain: Option<&TileTerrain>,
    origin: (f32, f32),
) -> Vec<FootLane> {
    let mut out = Vec::new();
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        if profile.paths.iter().any(|p| p.path_type == PATH_TYPE_RAIL) {
            let name = inst.sli_file.rsplit(['\\', '/']).next().unwrap_or("");
            out.extend(lanes_of_kind(inst, &profile, name, origin, LaneKind::Rail));
        }
    }
    let placed = tile
        .objects
        .iter()
        .map(|o| (o, None))
        .chain(attached.iter().map(|(o, z)| (o, Some(*z))));
    for (obj, attached_z) in placed {
        let Some((_, sco)) = job.sco(&obj.sco_file) else {
            continue;
        };
        if !sco.paths.iter().any(|p| p.path_type == PATH_TYPE_RAIL) {
            continue;
        }
        let absolute = sco.has_absolute_height();
        let ground = if absolute {
            0.0
        } else {
            terrain.map_or(0.0, |t| t.height(obj.position.x, obj.position.y))
        };
        let z = match (attached_z, absolute) {
            (Some(z), true) => z,
            _ => ground + obj.position.z,
        };
        let world = Vec3::new(origin.0 + obj.position.x, origin.1 + obj.position.y, z);
        if sco.switch_positions > 0 {
            let key = opts_sim::pedestrian::switches::switch_key(world.x, world.y);
            out.extend(opts_sim::pedestrian::lanes_from_points_object(
                &sco.paths,
                world,
                obj.heading_deg,
                PATH_TYPE_RAIL,
                key,
            ));
            continue;
        }
        out.extend(lanes_from_object_of_type(
            &sco.paths,
            world,
            obj.heading_deg,
            PATH_TYPE_RAIL,
        ));
    }
    out
}

/// Cycle-path lanes of a tile's splines in the session frame.
pub(in crate::in_game::omsi_world) fn tile_bike_lanes(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    origin: (f32, f32),
) -> Vec<FootLane> {
    let mut out = Vec::new();
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        let name = inst.sli_file.rsplit(['\\', '/']).next().unwrap_or("");
        out.extend(lanes_of_kind(inst, &profile, name, origin, LaneKind::Bike));
    }
    out
}

/// Registers a tile's cycle-path lanes.
pub(in crate::in_game::omsi_world) fn register_bike_tile(
    commands: &mut Commands,
    key: (i32, i32),
    lanes: Vec<FootLane>,
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<BikeNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's cycle-path lanes.
pub(in crate::in_game::omsi_world) fn unregister_bike_tile(
    commands: &mut Commands,
    key: (i32, i32),
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<BikeNetwork>() {
            net.0.remove_tile(key);
        }
    });
}

/// Registers a tile's track lanes.
pub(in crate::in_game::omsi_world) fn register_tile(
    commands: &mut Commands,
    key: (i32, i32),
    lanes: Vec<FootLane>,
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<RailNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's track lanes.
pub(in crate::in_game::omsi_world) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<RailNetwork>() {
            net.0.remove_tile(key);
        }
    });
}
