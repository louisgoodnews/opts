//! Spline attachments: street furniture, fences, shrubs and parking bays that a tile places
//! relative to a spline (`[splineAttachement]`, `[splineAttachement_repeater]`) rather than at
//! absolute coordinates.
//!
//! A single attachment's distance is a chainage from the start of the connected spline chain
//! that its carrying spline belongs to; a repeater's is measured from its carrying spline's own
//! start and may run on into the following splines. Positions are resolved against a map-wide
//! [`SplineIndex`] because chains cross tile borders.

use std::collections::{HashMap, HashSet};

use opts_bevy::scenery::spline_pose;
use opts_core::formats::map::{
    MapObjectInstance, MapSplineAttachment, MapSplineInstance, TileGrid,
};

use bevy::math::Vec3;

/// Upper bound on chain links followed in either direction (guards against cycles).
const MAX_CHAIN: usize = 512;

/// A spline with the tile it lies on.
type Placed = ((i32, i32), MapSplineInstance);

/// Every spline of a map by id code, with the tile it lies on.
#[derive(Default)]
pub struct SplineIndex {
    by_id: HashMap<u64, Placed>,
}

impl SplineIndex {
    /// Adds the splines of one tile.
    pub fn add_tile(&mut self, key: (i32, i32), splines: &[MapSplineInstance]) {
        for s in splines {
            self.by_id.insert(s.id_code, (key, s.clone()));
        }
    }

    /// Total length of the splines preceding `id` in its chain (following `conn_id_1`).
    fn chain_offset(&self, id: u64) -> f32 {
        let mut offset = 0.0;
        let mut seen = HashSet::from([id]);
        let mut cur = id;
        while let Some((_, s)) = self.by_id.get(&cur) {
            let prev = s.conn_id_1;
            let Some((_, p)) = self.by_id.get(&prev) else {
                break;
            };
            if !seen.insert(prev) || seen.len() > MAX_CHAIN {
                break;
            }
            offset += p.length_m;
            cur = prev;
        }
        offset
    }

    /// Resolves arc length `s` on spline `id`, walking into neighbouring splines when it lies
    /// outside the segment. Returns the spline, its tile and the local arc length.
    fn locate(&self, id: u64, mut s: f32) -> Option<(&Placed, f32)> {
        let mut cur = self.by_id.get(&id)?;
        for _ in 0..MAX_CHAIN {
            if s < 0.0 {
                let prev = self.by_id.get(&cur.1.conn_id_1)?;
                s += prev.1.length_m;
                cur = prev;
            } else if s > cur.1.length_m {
                let next = self.by_id.get(&cur.1.conn_id_2)?;
                s -= cur.1.length_m;
                cur = next;
            } else {
                return Some((cur, s));
            }
        }
        None
    }
}

/// Turns the attachments of a tile into object placements in that tile's local frame.
///
/// `elevation_at(x, y)` is the ground height the object loop will add to `position.z`, so the
/// returned instances land exactly `height_m` above the spline surface. Each instance comes with
/// that surface-relative elevation as an absolute height, for objects that ignore the terrain.
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
    for att in attachments {
        let Some(carrier) = tile_splines.get(att.spline_index) else {
            tracing::debug!(sco = %att.sco_file, spline = att.spline_index, "attachment names a spline the tile lacks");
            continue;
        };
        let first = match att.repeat {
            None => att.distance_m - index.chain_offset(carrier.id_code),
            Some(_) => att.distance_m,
        };
        let (count, step) = match att.repeat {
            Some((span, n)) if n > 0 => (n, span / n as f32),
            _ => (1, 0.0),
        };
        for i in 0..count {
            let s = first + step * i as f32;
            let Some((entry, local)) = index.locate(carrier.id_code, s) else {
                continue;
            };
            // The carrier may lie on a neighbouring tile: its frame's corner in this tile's frame.
            let offset = grid.neighbour_offset(tile, entry.0);
            let pose = spline_pose(&entry.1, local);
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
                    string_vars: Vec::new(),
                },
                absolute_z,
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spline(id: u64, prev: u64, next: u64, x: f32, len: f32) -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: String::new(),
            id_code: id,
            conn_id_1: prev,
            conn_id_2: next,
            position: Vec3::new(x, 10.0, 0.0),
            heading_deg: 90.0,
            length_m: len,
            radius_m: 0.0,
            gradient_start: 0.0,
            gradient_end: 0.0,
            has_height_profile: false,
            mirrored: false,
            raw_params: vec![],
        }
    }

    fn attachment(
        index: usize,
        lateral: f32,
        d: f32,
        repeat: Option<(f32, u32)>,
    ) -> MapSplineAttachment {
        MapSplineAttachment {
            sco_file: "a.sco".into(),
            id_code: 1,
            spline_index: index,
            lateral_m: lateral,
            height_m: 0.25,
            distance_m: d,
            heading_deg: 180.0,
            repeat,
            raw_params: vec![],
        }
    }

    fn chain() -> (Vec<MapSplineInstance>, SplineIndex) {
        let splines = vec![spline(1, 0, 2, 0.0, 50.0), spline(2, 1, 0, 50.0, 100.0)];
        let mut index = SplineIndex::default();
        index.add_tile((0, 0), &splines);
        (splines, index)
    }

    #[test]
    fn single_attachment_distance_is_chain_chainage() {
        let (splines, index) = chain();
        // 70 m along the chain = 20 m into the second spline; 3 m to the right of an east-bound road.
        let out = expand_attachments(
            &[attachment(1, 3.0, 70.0, None)],
            &splines,
            &index,
            (0, 0),
            &TileGrid::default(),
            |_, _| 0.0,
        );
        assert_eq!(out.len(), 1);
        assert!((out[0].0.position.x - 70.0).abs() < 1e-3);
        assert!((out[0].0.position.y - 7.0).abs() < 1e-3);
        assert!((out[0].0.position.z - 0.25).abs() < 1e-4);
        assert!((out[0].0.heading_deg - 270.0).abs() < 1e-3);
    }

    #[test]
    fn repeater_runs_on_into_following_spline() {
        let (splines, index) = chain();
        let out = expand_attachments(
            &[attachment(0, 0.0, 10.0, Some((120.0, 3)))],
            &splines,
            &index,
            (0, 0),
            &TileGrid::default(),
            |_, _| 0.0,
        );
        let xs: Vec<f32> = out.iter().map(|o| o.0.position.x).collect();
        assert_eq!(xs.len(), 3);
        assert!(
            (xs[0] - 10.0).abs() < 1e-3
                && (xs[1] - 50.0).abs() < 1e-3
                && (xs[2] - 90.0).abs() < 1e-3
        );
    }

    #[test]
    fn attachment_past_chain_end_is_dropped() {
        let (splines, index) = chain();
        let out = expand_attachments(
            &[attachment(1, 0.0, 400.0, None)],
            &splines,
            &index,
            (0, 0),
            &TileGrid::default(),
            |_, _| 0.0,
        );
        assert!(out.is_empty());
    }
}
