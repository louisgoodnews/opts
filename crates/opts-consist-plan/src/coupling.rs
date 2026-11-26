//! The coupling points that join two units of a train (moved from opts-world-load's
//! `vehicle/trailers.rs` with the planner, ISS-093).

use bevy::prelude::Vec3;
use opts_core::formats::vehicles::BusManifest;

/// The two coupling points that join a unit to the one in front of it, in the frame of the
/// train (y forward): the rear point of the unit in front and the front point of the unit behind,
/// each relative to its own vehicle's origin. A turned unit has its points negated and swapped
/// (its model's front point faces the rear of the train). `None` when a point is missing.
pub fn coupling_joint(
    parent: &BusManifest,
    parent_turned: bool,
    own: &BusManifest,
    own_turned: bool,
) -> Option<(Vec3, Vec3)> {
    let rear_of_parent = if parent_turned {
        parent.coupling_front.map(|c| -c)
    } else {
        parent.coupling_back
    }?;
    let front_of_own = if own_turned {
        own.coupling_back.map(|c| -c)
    } else {
        own.coupling_front
    }?;
    Some((rear_of_parent, front_of_own))
}
