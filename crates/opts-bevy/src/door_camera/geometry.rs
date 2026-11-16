//! Door geometry: the camera pose at a door, the door variable names and the nearest door point to a leaf.

use super::*;

/// The pose of a door camera in the vehicle's frame: stuck to the door, above its upper left
/// corner as seen from outside (towards the rear for a door on the right, towards the front on
/// the left), a little outside, looking down into the doorway.
#[must_use]
pub fn door_pose(door: &DoorPoint) -> Transform {
    let side = f32::from(door.side);
    // Bevy: x right, y up, z backward.
    let eye = door.position + Vec3::new(side * EYE_OUT_M, EYE_HEIGHT_M, side * EYE_ALONG_M);
    let target = door.position + Vec3::new(-side * 0.2, 0.7, 0.0);
    Transform::from_translation(eye).looking_at(target, Vec3::Y)
}

/// Whether a variable name is the position of a door leaf: it names a door and none of the
/// auxiliaries (alpha, warnings, speeds, triggers, sounds, requests, timers, ...).
#[must_use]
pub fn is_door_variable(name: &str) -> bool {
    let n = name.to_lowercase();
    let door = n.contains("door") || n.contains("tuer") || n.contains("tür");
    const AUX: [&str; 19] = [
        "taster",
        "alpha",
        "warn",
        "speed",
        "geschwindigkeit",
        "trigger",
        "sound",
        "req",
        "timer",
        "light",
        "bt_",
        "cp_",
        "hit",
        "stop",
        "free",
        "freigabe",
        "vorwahl",
        "tuer_auf",
        "tuer_zu",
    ];
    door && !AUX.iter().any(|a| n.contains(a))
}

/// The point nearest to `leaf`, by distance in the floor plane (Bevy axes).
#[must_use]
pub fn nearest_point(points: &[Vec3], leaf: Vec3) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a.x - leaf.x).hypot(a.z - leaf.z);
            let db = (b.x - leaf.x).hypot(b.z - leaf.z);
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
}
