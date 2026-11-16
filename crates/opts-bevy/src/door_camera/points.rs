//! Door points: the zone of a door along the vehicle and the points read from the passenger cabin.

use super::*;


/// Which end of the vehicle a door is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// The forward third of the doors' extent.
    Front,
    /// Between.
    Middle,
    /// The rear third.
    Rear,
}

/// One door of the vehicle.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorPoint {
    /// The pathpoint is an `[entry]` (else an `[exit]`).
    pub entry: bool,
    /// The pathpoint index in `paths.cfg`.
    pub index: usize,
    /// Floor position at the door, in the vehicle's frame (Bevy axes).
    pub position: Vec3,
    /// `1` for a door on the right side, `-1` on the left.
    pub side: i8,
    /// The unit the door is in (the entity its position is relative to); `None`: the head.
    pub unit: Option<Entity>,
}

/// The doors of a vehicle (on its root).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct DoorPoints(pub Vec<DoorPoint>);

impl DoorPoints {
    /// Where the door is along the vehicle compared to the other doors.
    #[must_use]
    pub fn zone(&self, door: usize) -> Zone {
        let Some(d) = self.0.get(door) else {
            return Zone::Middle;
        };
        // Bevy: forward is -Z, so the smaller z is further front.
        let (front, rear) = self
            .0
            .iter()
            .filter(|p| p.unit == d.unit)
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                (lo.min(p.position.z), hi.max(p.position.z))
            });
        let span = rear - front;
        if span < 1.0 {
            return Zone::Middle;
        }
        let t = (d.position.z - front) / span;
        if t < 1.0 / 3.0 {
            Zone::Front
        } else if t > 2.0 / 3.0 {
            Zone::Rear
        } else {
            Zone::Middle
        }
    }
}
