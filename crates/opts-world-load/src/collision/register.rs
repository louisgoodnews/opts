//! Registers a placed object's collision meshes (and its plate surfaces) with the indexes, and drops
//! a tile's colliders.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_core::formats::scenery::BaseSurface;

use super::super::ground_height::GroundIndex;
use super::super::WorldLoadJob;
use super::index::CollisionIndex;
use opts_collision::Obb;

/// Registers the collision meshes of a placed object (called while its tile is built).
pub(in super::super) fn register_object(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    sco: &opts_core::formats::scenery::SceneryObject,
    model_dir: &Path,
    tf: &Transform,
    key: (i32, i32),
    bend: Option<&(PathBuf, Arc<BaseSurface>)>,
) {
    if sco.no_collision || sco.collision_meshes.is_empty() {
        return;
    }
    let yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
    let heading = -yaw;
    let mut boxes = Vec::new();
    for rel in &sco.collision_meshes {
        let Some(parts) = job.collision_bounds(model_dir, rel, bend) else {
            continue;
        };
        for (lo, hi) in parts {
            let local_centre = (lo + hi) * 0.5;
            let half = (hi - lo) * 0.5;
            let c = tf.transform_point(local_centre);
            boxes.push(Obb::new(
                Vec2::new(c.x, -c.z),
                Vec2::new(half.x, half.z),
                heading,
                tf.translation.y + lo.y,
                tf.translation.y + hi.y,
            ));
        }
    }
    if boxes.is_empty() {
        return;
    }
    // An object marked `[surface]` is a drivable ground mesh: its collision mesh is what the
    // vehicles drive on (a junction or street plate a few decimetres over the terrain), not a
    // wall; it went into the obstacle index and stopped buses at the first plate (Köln,
    // Frankenstraße).
    if sco.surface {
        // The plate's textures under the wheels (M-0146).
        let tris = super::super::plate_surf::plate_triangles(job, sco, model_dir, tf, bend);
        commands.queue(move |world: &mut World| {
            if let Some(mut ground) = world.get_resource_mut::<GroundIndex>() {
                ground.insert_plates(key, boxes);
                ground.insert_plate_triangles(key, tris);
            }
        });
        return;
    }
    commands.queue(move |world: &mut World| {
        if let Some(mut index) = world.get_resource_mut::<CollisionIndex>() {
            for b in boxes {
                index.grid.insert(key, b);
            }
        }
    });
}

/// Drops a tile's colliders.
pub(in super::super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut index) = world.get_resource_mut::<CollisionIndex>() {
            index.grid.remove(&key);
        }
    });
}
