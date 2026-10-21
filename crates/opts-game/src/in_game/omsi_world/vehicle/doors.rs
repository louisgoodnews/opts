//! The doors of the player's vehicle for the door camera (M-0109).

use opts_bevy::door_camera::{DoorPoint, DoorPoints};
use opts_core::formats::text::{decode_bytes, scan_blocks};
use opts_core::formats::vehicles::PassengerPaths;

use super::*;

/// The door points of a cabin: `cabin_text` names entries and exits, `paths` has their positions.
/// Entries come first, then exits, each in declaration order; an index without a pathpoint is
/// left out.
#[must_use]
pub(in super::super::super) fn door_points(
    cabin_text: &str,
    paths: &PassengerPaths,
) -> Vec<DoorPoint> {
    let blocks = scan_blocks(cabin_text);
    let mut out = Vec::new();
    for (keyword, entry) in [("entry", true), ("exit", false)] {
        for b in blocks.iter().filter(|b| b.keyword == keyword) {
            let Some(index) = b
                .parameters
                .first()
                .and_then(|p| p.trim().parse::<usize>().ok())
            else {
                continue;
            };
            let Some(p) = paths.points.get(index) else {
                continue;
            };
            // Vehicle space (x right, y forward, z up) to Bevy axes.
            let position = opts_bevy::coords::omsi_to_bevy_pos(p.position);
            out.push(DoorPoint {
                entry,
                index,
                position,
                side: if position.x < -0.05 { -1 } else { 1 },
                unit: None,
            });
        }
    }
    out
}

/// [`door_points`] from the bytes of the two files (none when the paths file is unreadable).
#[must_use]
pub(in super::super::super) fn door_points_from_bytes(
    cabin: &[u8],
    paths: &[u8],
) -> Vec<DoorPoint> {
    let Ok(paths) = PassengerPaths::parse(&decode_bytes(paths)) else {
        return Vec::new();
    };
    door_points(&decode_bytes(cabin), &paths)
}

/// Reads the passenger cabin and the paths file of the vehicle and puts the door points on its
/// root; a vehicle without cabin or paths gets none.
pub(super) fn attach_door_points(
    job: &WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    manifest: &BusManifest,
    bus_dir: &Path,
) {
    let read = |rel: Option<&str>| {
        rel.and_then(|r| job.install.find_in(bus_dir, r))
            .and_then(|p| std::fs::read(p).ok())
    };
    let (Some(cabin), Some(paths)) = (
        read(manifest.passengercabin_cfg.as_deref()),
        read(manifest.paths_cfg.as_deref()),
    ) else {
        return;
    };
    let doors = door_points_from_bytes(&cabin, &paths);
    if !doors.is_empty() {
        tracing::info!(doors = doors.len(), "door camera points");
        commands.entity(root).insert(DoorPoints(doors));
    }
}

/// The door points of a unit behind the head, added to the head's: `unit` is the unit's entity,
/// the frame its points are relative to.
pub(super) fn attach_unit_door_points(
    job: &WorldLoadJob,
    commands: &mut Commands,
    head: Entity,
    unit: Entity,
    manifest: &BusManifest,
    dir: &Path,
) {
    let read = |rel: Option<&str>| {
        rel.and_then(|r| job.install.find_in(dir, r))
            .and_then(|p| std::fs::read(p).ok())
    };
    let (Some(cabin), Some(paths)) = (
        read(manifest.passengercabin_cfg.as_deref()),
        read(manifest.paths_cfg.as_deref()),
    ) else {
        return;
    };
    let mut doors = door_points_from_bytes(&cabin, &paths);
    if doors.is_empty() {
        return;
    }
    for d in &mut doors {
        d.unit = Some(unit);
    }
    commands.queue(move |world: &mut World| {
        if let Some(mut points) = world.get_mut::<DoorPoints>(head) {
            points.0.extend(doors);
        } else {
            world.entity_mut(head).insert(DoorPoints(doors));
        }
    });
}

#[cfg(test)]
mod tests;
