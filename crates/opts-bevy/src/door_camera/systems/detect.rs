//! Detecting open doors: which mesh animations show an open door leaf, and which door it is.

use super::*;

fn leaf_open_door_group(
    anim: &MeshAnimation,
    tf: &GlobalTransform,
    aabb: Option<&Aabb>,
    door_world: &[Vec3],
    doors: &[DoorPoint],
) -> Option<usize> {
    let step = anim
        .steps
        .iter()
        .zip(&anim.shown)
        .find(|(step, value)| is_door_variable(&step.variable) && value.abs() > OPEN_ABOVE)
        .map(|(step, _)| step)?;
    let pivot = step.frame.w_axis.truncate();
    let local = if pivot.length() >= 0.01 {
        Vec3::new(pivot.x, pivot.z, -pivot.y)
    } else {
        Vec3::from(aabb?.center)
    };
    let leaf = tf.transform_point(local);
    let i = nearest_point(door_world, leaf)
        .filter(|&i| (door_world[i].x - leaf.x).hypot(door_world[i].z - leaf.z) < LEAF_NEAR_M)?;
    Some(door_group(doors, i))
}

/// Looks at the door leaves a few times a second and keeps the door that moves.
pub fn detect_open_doors(
    time: Res<Time<Real>>,
    mut last: Local<f64>,
    mut state: ResMut<DoorCamera>,
    vehicles: Query<(Entity, &DoorPoints, Option<&crate::ScriptConsist>), With<PlayerVehicle>>,
    animated: Query<(&MeshAnimation, &GlobalTransform, Option<&Aabb>)>,
    globals: Query<&GlobalTransform>,
) {
    if !state.enabled {
        state.auto = None;
        state.open.clear();
        return;
    }
    let now = time.elapsed_secs_f64();
    if now - *last < DETECT_INTERVAL_S {
        return;
    }
    *last = now;
    let Some((vehicle, doors, consist)) = vehicles.iter().next() else {
        state.auto = None;
        state.open.clear();
        return;
    };
    let mut found = None;
    let mut now_open = Vec::new();
    let door_world: Vec<Vec3> = doors
        .0
        .iter()
        .map(|d| {
            globals
                .get(d.unit.unwrap_or(vehicle))
                .map_or(d.position, |g| g.transform_point(d.position))
        })
        .collect();
    for (anim, tf, aabb) in &animated {
        let ours = anim.owner == vehicle
            || consist.is_some_and(|c| c.units.iter().any(|u| u.part == Some(anim.owner)));
        if !ours {
            continue;
        }
        if let Some(d) = leaf_open_door_group(anim, tf, aabb, &door_world, &doors.0) {
            if !now_open.contains(&d) {
                now_open.push(d);
            }
            found = found.or(Some(d));
        }
    }
    now_open.sort_by_key(|d| state.open.iter().position(|o| o == d).unwrap_or(usize::MAX));
    if state.open != now_open {
        state.open = now_open;
    }
    match found {
        Some(door) => {
            state.auto = Some(door);
            state.auto_until = now + HOLD_S;
        }
        None if now > state.auto_until => state.auto = None,
        None => {}
    }
}

