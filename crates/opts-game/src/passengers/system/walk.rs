//! Walking the passengers to the doors of the bus.

use super::*;

/// People walking up to the doors; they board when the door is open.
pub(super) fn walk_to_doors(
    e: &mut PassengerEnv,
    rig: &mut PassengerRig,
    (frames, to_world, vehicle): (&[Affine3A], Affine3A, Entity),
    doors: &Doors,
    dt: f32,
) {
    let mut still = Vec::new();
    for mut w in std::mem::take(&mut rig.approaching) {
        w.target = rig.door_world(w.entry, frames);
        let d = w.position.distance(w.target);
        let step = OUTSIDE_SPEED_M_S * dt;
        if d > 0.25 {
            let dir = (w.target - w.position) / d;
            w.position += dir * step.min(d);
            if let Ok(mut t) = e.transforms.get_mut(w.entity) {
                t.translation = w.position;
                t.rotation = facing(dir);
            }
            if let Ok(h) = e.humans.get(w.entity) {
                w.phase = advance_phase(h, w.phase, OUTSIDE_SPEED_M_S, dt);
                apply_gait(h, w.leg_m, w.phase, &mut e.transforms);
            }
            still.push(w);
        } else if doors.entries.get(w.entry).copied().unwrap_or(false)
            && rig.sim.passengers().len() < rig.sim.capacity()
        {
            if let Some(id) = rig.sim.board(w.age, w.stops, w.entry) {
                // Beyond the figure budget a person boards unseen.
                if rig.actors.len() >= ACTOR_BUDGET {
                    e.commands.entity(w.entity).despawn();
                    continue;
                }
                let unit = rig
                    .sim
                    .passengers()
                    .iter()
                    .find(|p| p.id == id)
                    .map_or(0, |p| p.unit);
                let frame = frames.get(unit).copied().unwrap_or(to_world);
                let local = frame.inverse().transform_point3(w.position);
                let parent = rig.units.get(unit).map_or(vehicle, |u| u.root);
                e.commands.entity(w.entity).insert(ChildOf(parent));
                if let Ok(mut t) = e.transforms.get_mut(w.entity) {
                    t.translation = local;
                }
                rig.actors.insert(
                    id,
                    Shown {
                        unit,
                        entity: w.entity,
                        leg_m: w.leg_m,
                        phase: w.phase,
                        local,
                        seated: None,
                    },
                );
            } else {
                e.commands.entity(w.entity).despawn();
            }
        } else {
            // Waiting for the door: stand still.
            if let Ok(h) = e.humans.get(w.entity) {
                apply_rest_pose(h, HumanPose::Standing, &mut e.transforms);
            }
            still.push(w);
        }
    }
    rig.approaching = still;
}
