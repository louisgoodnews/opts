//! The footpath lanes of a tile and their registration with the pedestrian network.

use super::*;

/// Footpath lanes of a tile in the world frame: splines (`profile.paths`) and the paths of the
/// placed scenery objects.
pub(in crate) fn tile_lanes(
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
        if profile.paths.iter().any(|p| p.path_type == 1) {
            out.extend(lanes_from_spline(inst, &profile, origin));
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
        if !sco.paths.iter().any(|p| p.path_type == 1) {
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
        out.extend(lanes_from_object(&sco.paths, world, obj.heading_deg));
    }
    out
}

/// Registers a tile's lanes with the network.
pub(in crate) fn register_tile(
    commands: &mut Commands,
    key: (i32, i32),
    lanes: Vec<FootLane>,
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<PedestrianNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's lanes.
pub(in crate) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<PedestrianNetwork>() {
            net.0.remove_tile(key);
        }
    });
}
