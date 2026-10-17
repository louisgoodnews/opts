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
mod tests {
    use super::*;
    use opts_core::formats::map::RepeaterLink;

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

    fn attachment(index: usize, lateral: f32, d: f32) -> MapSplineAttachment {
        MapSplineAttachment {
            sco_file: "a.sco".into(),
            id_code: 1,
            spline_index: index,
            lateral_m: lateral,
            height_m: 0.25,
            distance_m: d,
            heading_deg: 180.0,
            field8: 0.0,
            field9: 0.0,
            spacing_m: 0.0,
            range_m: 0.0,
            field12: 0.0,
            repeater: None,
            string_vars: vec![],
        }
    }

    fn run(index: usize, d: f32, spacing: f32, range: f32) -> MapSplineAttachment {
        MapSplineAttachment {
            spacing_m: spacing,
            range_m: range,
            ..attachment(index, 0.0, d)
        }
    }

    fn index_of(tiles: &[((i32, i32), Vec<MapSplineInstance>)]) -> SplineIndex {
        let mut index = SplineIndex::default();
        for (key, splines) in tiles {
            index.add_tile(*key, splines);
        }
        index.finish();
        index
    }

    fn chain() -> (Vec<MapSplineInstance>, SplineIndex) {
        let splines = vec![spline(1, 0, 2, 0.0, 50.0), spline(2, 1, 0, 50.0, 100.0)];
        let index = index_of(&[((0, 0), splines.clone())]);
        (splines, index)
    }

    fn xs(out: &[(MapObjectInstance, f32)]) -> Vec<f32> {
        out.iter().map(|o| o.0.position.x).collect()
    }

    fn expand(
        atts: &[MapSplineAttachment],
        splines: &[MapSplineInstance],
        index: &SplineIndex,
        tile: (i32, i32),
    ) -> Vec<(MapObjectInstance, f32)> {
        expand_attachments(atts, splines, index, tile, &TileGrid::default(), |_, _| 0.0)
    }

    #[test]
    fn single_attachment_distance_is_chain_chainage() {
        let (splines, index) = chain();
        // 70 m along the chain = 20 m into the second spline; 3 m to the right of an east-bound road.
        let mut att = attachment(1, 3.0, 70.0);
        att.string_vars = vec![String::new(), "1".into()];
        let out = expand(&[att], &splines, &index, (0, 0));
        assert_eq!(out.len(), 1);
        assert!((out[0].0.position.x - 70.0).abs() < 1e-3);
        assert!((out[0].0.position.y - 7.0).abs() < 1e-3);
        assert!((out[0].0.position.z - 0.25).abs() < 1e-4);
        assert!((out[0].0.heading_deg - 270.0).abs() < 1e-3);
        assert_eq!(
            out[0].0.string_vars,
            vec!["", "1"],
            "string values are passed on"
        );
    }

    /// A master places `⌊range / spacing⌋ + 1` instances `spacing` apart along its chain.
    #[test]
    fn master_run_places_every_spacing_along_the_chain() {
        let (splines, index) = chain();
        let out = expand(&[run(0, 10.0, 20.0, 100.0)], &splines, &index, (0, 0));
        assert_eq!(xs(&out), vec![10.0, 30.0, 50.0, 70.0, 90.0, 110.0]);
        // range 140 / spacing 7 = 20 steps: 21 instances when the chain is long enough.
        let out = expand(&[run(0, 0.0, 7.0, 140.0)], &splines, &index, (0, 0));
        assert_eq!(out.len(), 21);
        // …but never past the chain's end (150 m): 0, 7, …, 147.
        let out = expand(&[run(0, 0.0, 7.0, 400.0)], &splines, &index, (0, 0));
        assert_eq!(out.len(), 22);
        assert!(xs(&out).iter().all(|x| (0.0..=150.0).contains(x)));
    }

    /// The master stops at its tile's border; the repeater on the next tile continues from its
    /// first instance, and a duplicate repeater adds nothing.
    #[test]
    fn repeater_continues_the_run_on_the_next_tile() {
        let a = vec![spline(1, 0, 2, 0.0, 50.0)];
        let b = vec![spline(2, 1, 0, 50.0, 100.0)];
        let index = index_of(&[((0, 0), a.clone()), ((1, 0), b.clone())]);
        let master = run(0, 10.0, 20.0, 100.0);
        let on_a = expand(std::slice::from_ref(&master), &a, &index, (0, 0));
        assert_eq!(xs(&on_a), vec![10.0, 30.0, 50.0]);
        let rep = MapSplineAttachment {
            repeater: Some(RepeaterLink {
                master_tile: 0,
                first_instance: 3,
            }),
            ..master
        };
        let on_b = expand(&[rep.clone(), rep], &b, &index, (1, 0));
        // Instances 3..=5 at chainages 70, 90, 110 = 20, 40, 60 m into spline 2 (tile frame x).
        assert_eq!(xs(&on_b), vec![70.0, 90.0, 110.0]);
    }

    #[test]
    fn chain_offsets_match_a_plain_walk_also_in_loops() {
        let splines = vec![
            spline(1, 0, 2, 0.0, 50.0),
            spline(2, 1, 3, 50.0, 100.0),
            spline(3, 2, 0, 150.0, 25.0),
            // A ring of three.
            spline(10, 12, 11, 0.0, 10.0),
            spline(11, 10, 12, 0.0, 20.0),
            spline(12, 11, 10, 0.0, 30.0),
        ];
        let index = index_of(&[((0, 0), splines.clone())]);
        for s in &splines {
            assert_eq!(
                index.chain_offset(s.id_code),
                index.walk_offset(s.id_code),
                "{}",
                s.id_code
            );
        }
        assert_eq!(index.chain_offset(3), 150.0);
        assert_eq!(index.chain_offset(10), 50.0);
    }

    #[test]
    fn attachment_past_chain_end_is_dropped() {
        let (splines, index) = chain();
        let out = expand(&[attachment(1, 0.0, 400.0)], &splines, &index, (0, 0));
        assert!(out.is_empty());
    }
}

/// Content-gated checks against the installed maps (skipped unless `OMSI_PATH` is set).
#[cfg(test)]
mod content {
    use super::*;
    use opts_core::formats::map::MapTile;
    use std::path::{Path, PathBuf};

    fn omsi() -> Option<PathBuf> {
        std::env::var_os("OMSI_PATH").map(PathBuf::from)
    }

    /// Every tile of a map folder with its key.
    fn tiles(map: &Path) -> Vec<((i32, i32), MapTile)> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(map).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            let Some(xy) = name
                .strip_prefix("tile_")
                .and_then(|n| n.strip_suffix(".map"))
            else {
                continue;
            };
            let Some((x, y)) = xy.split_once('_') else {
                continue;
            };
            let (Ok(x), Ok(y)) = (x.parse(), y.parse()) else {
                continue;
            };
            if let Some(t) = std::fs::read(e.path())
                .ok()
                .and_then(|b| MapTile::parse(&b).ok())
            {
                out.push(((x, y), t));
            }
        }
        out.sort_by_key(|(k, _)| *k);
        out
    }

    fn index_of(tiles: &[((i32, i32), MapTile)]) -> SplineIndex {
        let mut index = SplineIndex::default();
        for (key, t) in tiles {
            index.add_tile(*key, &t.splines);
        }
        index.finish();
        index
    }

    /// Berlin-Spandau tile 2404/11277: the parking bays of every run stand exactly `spacing`
    /// (7 m) apart along the chain, none outside `[d, d + range]`; map-wide counts are printed.
    #[test]
    fn spandau_parking_bays_follow_spacing_and_range() {
        let Some(root) = omsi() else {
            eprintln!("OMSI_PATH not set; skipping");
            return;
        };
        let map = root.join("maps").join("Berlin-Spandau");
        if !map.is_dir() {
            eprintln!("Berlin-Spandau not installed; skipping");
            return;
        }
        let tiles = tiles(&map);
        let index = index_of(&tiles);
        let (_, tile) = tiles
            .iter()
            .find(|(k, _)| *k == (2404, 11277))
            .expect("tile 2404/11277");
        let mut runs = 0;
        let mut bays = 0;
        for att in tile
            .spline_attachments
            .iter()
            .filter(|a| a.sco_file.to_ascii_lowercase().contains("car_park") && a.is_run())
        {
            let carrier = &tile.splines[att.spline_index];
            let sites = instance_sites(att, carrier, &index);
            assert!(
                (att.spacing_m - 7.0).abs() < 1e-3,
                "spacing {}",
                att.spacing_m
            );
            for s in &sites {
                let d = f64::from(att.distance_m);
                assert!(
                    s.chainage >= d - 1e-6 && s.chainage <= d + f64::from(att.range_m) + 1e-3,
                    "bay outside [d, d + range]: {} not in [{d}, {}]",
                    s.chainage,
                    d + f64::from(att.range_m)
                );
                assert!(s.local >= 0.0 && s.local <= f64::from(s.spline.1.length_m) + 1e-3);
            }
            for w in sites.windows(2) {
                assert_eq!(w[1].k, w[0].k + 1);
                assert!((w[1].chainage - w[0].chainage - 7.0).abs() < 1e-3);
            }
            runs += 1;
            bays += sites.len();
        }
        eprintln!("tile 2404/11277: {runs} parking runs, {bays} bays");
        assert!(runs > 0 && bays > runs, "{runs} runs, {bays} bays");

        // Map-wide: instances placed per kind (bays, lamps).
        let count = |needle: &str| -> usize {
            tiles
                .iter()
                .map(|(key, t)| {
                    let atts: Vec<MapSplineAttachment> = t
                        .spline_attachments
                        .iter()
                        .filter(|a| a.sco_file.to_ascii_lowercase().contains(needle))
                        .cloned()
                        .collect();
                    expand_attachments(
                        &atts,
                        &t.splines,
                        &index,
                        *key,
                        &TileGrid::default(),
                        |_, _| 0.0,
                    )
                    .len()
                })
                .sum()
        };
        let (cars, lights) = (count("car_park"), count("light"));
        eprintln!("Berlin-Spandau: {cars} parking bays, {lights} 'light' attachments placed");
        // The probe model gives 3,858 instances, 84 of them from 3 repeaters that repeat others.
        assert_eq!(cars, 3774, "parking bays");
        assert_eq!(lights, 1806, "'light' attachments");
    }

    /// Corpus-wide: a repeater's first instance `p2` lies on its own spline and instance `p2 − 1`
    /// does not, for at least 85 % of the repeaters (measured 87.7 %).
    #[test]
    fn repeater_first_instance_is_consistent_corpus_wide() {
        let Some(root) = omsi() else {
            eprintln!("OMSI_PATH not set; skipping");
            return;
        };
        let (mut total, mut first) = (0usize, 0usize);
        for e in std::fs::read_dir(root.join("maps"))
            .into_iter()
            .flatten()
            .flatten()
        {
            if !e.path().join("global.cfg").is_file() {
                continue;
            }
            let tiles = tiles(&e.path());
            if !tiles
                .iter()
                .any(|(_, t)| t.spline_attachments.iter().any(|a| a.repeater.is_some()))
            {
                continue;
            }
            let index = index_of(&tiles);
            for (_, t) in &tiles {
                for a in &t.spline_attachments {
                    let Some(link) = a.repeater else { continue };
                    total += 1;
                    let Some(s) = t.splines.get(a.spline_index) else {
                        continue;
                    };
                    let off = index.chain_offset(s.id_code);
                    let len = f64::from(s.length_m) + 0.05;
                    let on = |k: u32| (-0.05..=len).contains(&(a.chainage(k) - off));
                    let k = link.first_instance;
                    if on(k) && (k == 0 || !on(k - 1)) {
                        first += 1;
                    }
                }
            }
        }
        eprintln!("repeaters: {first} of {total} start on their own spline");
        if total > 0 {
            assert!(first as f64 >= 0.85 * total as f64, "{first} of {total}");
        }
    }
}
