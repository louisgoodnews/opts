//! Spline attachments: street furniture, fences, shrubs and parking bays that a tile places
//! relative to a spline (`[splineAttachement]`, `[splineAttachement_repeater]`) rather than at
//! absolute coordinates.
//!
//! An attachment stands for a run of instances at the chainages `d + k · spacing`,
//! `k = 0 … ⌊range / spacing⌋`, measured from the start of the connected spline chain its carrier
//! belongs to. A master places the instances that fall on the splines of its own tile, following
//! the chain from its carrier on; a repeater continues the run on another tile from instance `p2`
//! on (`docs/src/formats/map/tiles.md`). Chain offsets come from a map-wide [`SplineIndex`] built
//! once, so placements do not depend on which tiles are resident.

use std::collections::HashSet;

use opts_bevy::scenery::spline_pose;
use opts_core::formats::map::{
    MapObjectInstance, MapSplineAttachment, MapSplineInstance, TileGrid,
};

use bevy::math::Vec3;

mod index;
mod sites;
pub use index::SplineIndex;
pub use sites::{instance_sites, InstanceSite};

/// Upper bound on chain links followed in either direction (guards against cycles).
const MAX_CHAIN: usize = 4096;
/// Slack when deciding on which spline of a chain an instance lies (m).
const CHAIN_EPS_M: f64 = 1e-4;

/// A spline with the tile it lies on.
type Placed = ((i32, i32), MapSplineInstance);

/// Turns the attachments of a tile into object placements in that tile's local frame.
///
/// `elevation_at(x, y)` is the ground height the object loop will add to `position.z`, so the
/// returned instances land exactly `height_m` above the spline surface. Each instance comes with
/// that surface-relative elevation as an absolute height, for objects that ignore the terrain.
/// Repeaters that repeat each other (same id, spline and first instance) are placed once.
#[must_use]
pub fn expand_attachments(
    attachments: &[MapSplineAttachment],
    tile_splines: &[MapSplineInstance],
    index: &SplineIndex,
    tile: (i32, i32),
    grid: &TileGrid,
    elevation_at: impl Fn(f32, f32) -> f32,
) -> Vec<(MapObjectInstance, f32)> {
    let mut out = Vec::new();
    let mut repeaters = HashSet::new();
    for att in attachments {
        let Some(carrier) = tile_splines.get(att.spline_index) else {
            tracing::debug!(sco = %att.sco_file, spline = att.spline_index, "attachment names a spline the tile lacks");
            continue;
        };
        if let Some(link) = att.repeater {
            if !repeaters.insert((att.id_code, att.spline_index, link.first_instance)) {
                continue;
            }
        }
        for site in instance_sites(att, carrier, index) {
            let entry = site.spline;
            // The carrier may lie on a neighbouring tile: its frame's corner in this tile's frame.
            let offset = grid.neighbour_offset(tile, entry.0);
            let pose = spline_pose(&entry.1, site.local as f32);
            let (rx, ry) = (pose.heading.cos(), -pose.heading.sin());
            let x = pose.x + rx * att.lateral_m + offset.x;
            let y = pose.y + ry * att.lateral_m + offset.y;
            let absolute_z = pose.z + att.height_m;
            out.push((
                MapObjectInstance {
                    flag: 0,
                    sco_file: att.sco_file.clone(),
                    id_code: att.id_code,
                    position: Vec3::new(x, y, pose.z + att.height_m - elevation_at(x, y)),
                    heading_deg: pose.heading.to_degrees() + att.heading_deg,
                    pitch_deg: None,
                    roll_deg: None,
                    string_vars: att.string_vars.clone(),
                },
                absolute_z,
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests;
