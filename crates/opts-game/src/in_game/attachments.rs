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

use std::collections::{HashMap, HashSet};

use opts_bevy::scenery::spline_pose;
use opts_core::formats::map::{
    MapObjectInstance, MapSplineAttachment, MapSplineInstance, TileGrid,
};

use bevy::math::Vec3;

/// Upper bound on chain links followed in either direction (guards against cycles).
const MAX_CHAIN: usize = 4096;
/// Slack when deciding on which spline of a chain an instance lies (m).
const CHAIN_EPS_M: f64 = 1e-4;

/// A spline with the tile it lies on.
type Placed = ((i32, i32), MapSplineInstance);

/// Every spline of a map by id code, with the tile it lies on and its chain offset.
#[derive(Default)]
pub struct SplineIndex {
    by_id: HashMap<u64, Placed>,
    /// Total length of the splines preceding each spline in its chain (following `conn_id_1`).
    offsets: HashMap<u64, f64>,
}

impl SplineIndex {
    /// Adds the splines of one tile. Call [`Self::finish`] after the last tile.
    pub fn add_tile(&mut self, key: (i32, i32), splines: &[MapSplineInstance]) {
        for s in splines {
            self.by_id.insert(s.id_code, (key, s.clone()));
        }
        self.offsets.clear();
    }

    /// Number of splines indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no spline is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Computes the chain offsets of all splines (each chain is walked once).
    pub fn finish(&mut self) {
        let ids: Vec<u64> = self.by_id.keys().copied().collect();
        let mut offsets: HashMap<u64, f64> = HashMap::with_capacity(ids.len());
        for id in ids {
            if offsets.contains_key(&id) {
                continue;
            }
            // Walk back to the chain's head or to a spline whose offset is known.
            let mut path = vec![id];
            let mut seen = HashSet::from([id]);
            let mut base = Some(0.0);
            loop {
                let cur = *path.last().unwrap_or(&id);
                let prev = self.by_id.get(&cur).map_or(0, |(_, s)| s.conn_id_1);
                let Some((_, p)) = self.by_id.get(&prev) else {
                    break;
                };
                if let Some(o) = offsets.get(&prev) {
                    base = Some(o + f64::from(p.length_m));
                    break;
                }
                if !seen.insert(prev) || path.len() >= MAX_CHAIN {
                    // A loop (or an absurdly long chain): walk each spline on its own.
                    base = None;
                    break;
                }
                path.push(prev);
            }
            match base {
                Some(mut o) => {
                    for (i, &sid) in path.iter().enumerate().rev() {
                        if i + 1 < path.len() {
                            o += self
                                .by_id
                                .get(&path[i + 1])
                                .map_or(0.0, |(_, s)| f64::from(s.length_m));
                        }
                        offsets.insert(sid, o);
                    }
                }
                None => {
                    for &sid in &path {
                        offsets.insert(sid, self.walk_offset(sid));
                    }
                }
            }
        }
        self.offsets = offsets;
    }

    /// Chain offset by walking `conn_id_1` until the head or a spline already passed.
    fn walk_offset(&self, id: u64) -> f64 {
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
            offset += f64::from(p.length_m);
            cur = prev;
        }
        offset
    }

    /// Total length of the splines preceding `id` in its chain.
    #[must_use]
    pub fn chain_offset(&self, id: u64) -> f64 {
        self.offsets
            .get(&id)
            .copied()
            .unwrap_or_else(|| self.walk_offset(id))
    }

    /// Resolves arc length `s` on spline `id`, walking into neighbouring splines when it lies
    /// outside the segment. Returns the spline, its tile and the local arc length.
    fn locate(&self, id: u64, mut s: f64) -> Option<(&Placed, f64)> {
        let mut cur = self.by_id.get(&id)?;
        for _ in 0..MAX_CHAIN {
            let len = f64::from(cur.1.length_m);
            if s < -CHAIN_EPS_M {
                let prev = self.by_id.get(&cur.1.conn_id_1)?;
                s += f64::from(prev.1.length_m);
                cur = prev;
            } else if s > len + CHAIN_EPS_M {
                let next = self.by_id.get(&cur.1.conn_id_2)?;
                s -= len;
                cur = next;
            } else {
                return Some((cur, s.clamp(0.0, len)));
            }
        }
        None
    }
}

/// Where one instance of an attachment lies.
pub struct InstanceSite<'a> {
    /// Instance index `k`.
    pub k: u32,
    /// Chainage from the start of the carrier's chain (m).
    pub chainage: f64,
    /// The spline it lies on, with its tile.
    pub spline: &'a Placed,
    /// Arc length on that spline (m).
    pub local: f64,
}

/// The instances one attachment places, in instance order.
///
/// A single attachment (no run) is located along its chain wherever that leads, as before; a run
/// places the instances from its first index on along the chain from its carrier, while the chain
/// stays on the carrier's tile.
#[must_use]
pub fn instance_sites<'a>(
    att: &MapSplineAttachment,
    carrier: &MapSplineInstance,
    index: &'a SplineIndex,
) -> Vec<InstanceSite<'a>> {
    let range = att.instances();
    let offset = index.chain_offset(carrier.id_code);
    if !att.is_run() {
        let k = *range.start();
        let chainage = att.chainage(k);
        return index
            .locate(carrier.id_code, chainage - offset)
            .map(|(spline, local)| InstanceSite {
                k,
                chainage,
                spline,
                local,
            })
            .into_iter()
            .collect();
    }
    let Some(home) = index.by_id.get(&carrier.id_code).map(|(key, _)| *key) else {
        return Vec::new();
    };
    let (mut k, last) = (*range.start(), *range.end());
    let mut out = Vec::new();
    let mut acc = offset;
    let mut cur = carrier.id_code;
    let mut seen = HashSet::new();
    while k <= last && seen.insert(cur) && seen.len() <= MAX_CHAIN {
        let Some(placed) = index.by_id.get(&cur) else {
            break;
        };
        if placed.0 != home {
            break;
        }
        let len = f64::from(placed.1.length_m.max(0.0));
        while k <= last {
            let chainage = att.chainage(k);
            if chainage > acc + len + CHAIN_EPS_M {
                break;
            }
            let local = chainage - acc;
            if local >= -CHAIN_EPS_M {
                out.push(InstanceSite {
                    k,
                    chainage,
                    spline: placed,
                    local: local.clamp(0.0, len),
                });
            }
            k += 1;
        }
        acc += len;
        cur = placed.1.conn_id_2;
    }
    out
}

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
