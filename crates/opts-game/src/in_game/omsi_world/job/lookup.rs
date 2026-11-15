//! Lookups on the load job: grid and tile positions, start entries, vehicle files and the parking list.

use super::*;

/// How far behind a bus stop object a vehicle starting there stands when its `.bus` gives no
/// bounding box (m): about half a 12 m bus, so its front is at the stop.
pub(in super::super::super) const STOP_SPAWN_BACK_M: f32 = 6.0;

/// The distance from a vehicle's origin to its front (m): the `.bus` bounding box's centre plus
/// half its length along the vehicle (`y`). For an articulated bus that is the front unit.
#[must_use]
pub(in super::super::super) fn front_of(
    bus: &opts_core::formats::vehicles::BusManifest,
) -> Option<f32> {
    bus.bounding_box
        .map(|(size, centre)| centre.y + size.y * 0.5)
        .filter(|f| f.is_finite() && (0.5..30.0).contains(f))
}

mod files;
mod places;
mod spawn;
mod tiles;
