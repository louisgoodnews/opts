//! The exhaust outlet of a vehicle: the model's own emitter when it defines one, else the fallback
//! behind and above the vehicle (M-0117, ISS-038).

use bevy::prelude::*;
use opts_bevy::vehicle::ExhaustOutlet;

/// Fallback distance (m) of the exhaust outlet behind the vehicle's origin.
pub const REAR_OFFSET_M: f32 = 5.5;
/// Fallback height (m) of the exhaust outlet above the vehicle's origin.
pub const OUTLET_HEIGHT_M: f32 = 1.0;

/// The exhaust outlet of a vehicle at `vehicle`: the model's own outlet `data` when it defines
/// one, else the fallback.
#[must_use]
pub fn outlet_of(vehicle: &GlobalTransform, data: Option<&ExhaustOutlet>) -> Vec3 {
    match data {
        Some(ExhaustOutlet(local)) => vehicle.transform_point(*local),
        None => fallback_outlet_of(vehicle),
    }
}

/// The outlet of a vehicle with `units` behind it (its articulated trailers, front to rear): the
/// rear-most unit that defines an emitter, else the head's own `data` or the fallback.
#[must_use]
pub fn chain_outlet<'a>(
    vehicle: &GlobalTransform,
    data: Option<&ExhaustOutlet>,
    units: impl DoubleEndedIterator<Item = (&'a GlobalTransform, Option<&'a ExhaustOutlet>)>,
) -> Vec3 {
    units
        .rev()
        .find_map(|(unit, emitter)| Some(unit.transform_point(emitter?.0)))
        .unwrap_or_else(|| outlet_of(vehicle, data))
}

/// The fallback outlet of a vehicle at `vehicle`: behind it and above the ground.
#[must_use]
pub fn fallback_outlet_of(vehicle: &GlobalTransform) -> Vec3 {
    let t = vehicle.compute_transform();
    t.translation - t.forward() * REAR_OFFSET_M + Vec3::Y * OUTLET_HEIGHT_M
}
