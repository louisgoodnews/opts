//! Boarding and leaving: passengers sent home, and passengers who board while no one watches.

use super::*;

/// Switched off: everybody goes at once.
pub(super) fn send_everybody_home(e: &mut PassengerEnv, rig: &mut PassengerRig) {
    for (_, s) in rig.actors.drain() {
        e.commands.entity(s.entity).despawn();
    }
    for w in rig.approaching.drain(..).chain(rig.leaving.drain(..)) {
        e.commands.entity(w.entity).despawn();
    }
    rig.sim.clear();
    rig.stop = None;
}

/// The rest of the crowd boards through the open doors, one person per door in turn, while there
/// is room and the bus takes their destination.
pub(super) fn board_unseen(
    rig: &mut PassengerRig,
    doors: &Doors,
    dt: f32,
    bus: Option<&Destination>,
) {
    rig.unseen_timer += dt;
    if rig.unseen_timer >= UNSEEN_BOARD_S && !rig.unseen.is_empty() {
        rig.unseen_timer = 0.0;
        for entry in 0..doors.entries.len() {
            if !doors.entries[entry] || rig.sim.passengers().len() >= rig.sim.capacity() {
                continue;
            }
            let found = rig
                .unseen
                .iter()
                .position(|u| u.entry == entry && admits(u.dest.as_ref(), bus));
            if let Some(i) = found {
                let u = rig.unseen.remove(i);
                rig.sim.board_to(u.age, u.stops, u.entry, u.dest);
            }
        }
    }
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod tests;
