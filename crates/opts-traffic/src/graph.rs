//! The directed lane graph built from OMSI `[path]` records.
//!
//! OMSI never names a path's successor: **connectivity is geometric** — a lane continues into any
//! lane of the same kind whose start coincides with its end in position and heading
//! (`docs/src/formats/text/sco.md`, "Path connectivity"). [`LaneGraph::build`] does exactly that
//! with a spatial hash, after expanding every segment into directed lanes (`direction` `1` is
//! stored reversed, `2` yields two lanes).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::OnceLock;

use glam::DVec2;
use thiserror::Error;

use crate::geometry::PathGeometry;
use crate::rng::Rng;

/// Index of a lane in a [`LaneGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LaneId(pub u32);

/// What may use a path (`[path]` type `0`/`1`/`2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LaneKind {
    /// AI road traffic.
    Road,
    /// AI pedestrians.
    Pedestrian,
    /// Rail vehicles (AI or player).
    Rail,
}

impl LaneKind {
    /// From the `[path]` type number.
    #[must_use]
    pub fn from_path_type(t: u32) -> Option<Self> {
        match t {
            0 => Some(Self::Road),
            1 => Some(Self::Pedestrian),
            2 => Some(Self::Rail),
            _ => None,
        }
    }
}

/// Allowed travel direction of a path (`0` forward only, `1` reverse only, `2` both).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Along the path's start→end direction.
    #[default]
    Forward,
    /// Against it.
    Reverse,
    /// Both ways.
    Both,
}

impl Direction {
    /// From the `[path]` direction number (unknown values count as forward).
    #[must_use]
    pub fn from_number(n: u32) -> Self {
        match n {
            1 => Self::Reverse,
            2 => Self::Both,
            _ => Self::Forward,
        }
    }

    /// The direction seen from the other end of the path.
    #[must_use]
    pub fn reversed(self) -> Self {
        match self {
            Self::Forward => Self::Reverse,
            Self::Reverse => Self::Forward,
            Self::Both => Self::Both,
        }
    }
}

/// Traffic rules of a path (`[rule]` records of the owning spline or object).
///
/// Vehicle-type flags (the corpus has them only with value `0`; not confirmed in OMSI ❓): `bus`
/// makes the lane a lane **for buses**, `no_cars` keeps **cars** off it, and `trucks` marks a lane
/// **trucks may use**. Read as a whitelist (project lead, 2026-10-19): a lane with a type rule is
/// for the named types only (`bus` → buses only, `trucks` → trucks only). Trucks prefer `trucks`
/// lanes; the rules never apply to the player.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaneRules {
    /// `trafficdensity` rules by vehicle group (the rule's flag; `0` = none for that group;
    /// corpus values `0`, `0.001`, `0.1`, `0.5`, `1`, `3`, …).
    pub density: crate::density::LaneDensity,
    /// `priority`: raw value (corpus: `64` and `192`; the order is not settled ❓).
    pub priority: Option<u32>,
    /// A `trucks` rule exists (trucks may use the lane, and prefer it).
    pub trucks: bool,
    /// A `bus` rule exists.
    pub bus: bool,
    /// A `no_cars` rule exists.
    pub no_cars: bool,
    /// An `overtaking_prohib` rule exists.
    pub no_overtaking: bool,
}

impl LaneRules {
    /// Whether vehicles of `class` may use the lane. The vehicle-type rules are a whitelist
    /// ("only the named types are allowed here"): with a `bus` and/or `trucks` rule only those
    /// types may use the lane; `no_cars` additionally keeps cars off. Applies to AI vehicles
    /// only — the player's vehicle is never stopped by a rule.
    #[must_use]
    pub fn allows(&self, class: crate::idm::VehicleClass) -> bool {
        use crate::idm::VehicleClass::{Bus, Car, Truck};
        if class == Car && self.no_cars {
            return false;
        }
        if self.bus || self.trucks {
            return (class == Bus && self.bus) || (class == Truck && self.trucks);
        }
        true
    }
}

/// Height profile along a lane: start height and the gradients (rise over run) at both ends.
/// Between them the gradient changes linearly (`[spline]` records:
/// `rise = length · (g0 + g1) / 2`) or, when `rise` is given (`[spline_h]`), the height follows
/// the cubic Hermite ramp of the road mesh (`opts_core::formats::map::spline_height_offset`,
/// `docs/src/formats/map/tiles.md`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Elevation {
    /// Height at the start in metres.
    pub z0: f64,
    /// Gradient at the start (ratio).
    pub grad_start: f64,
    /// Gradient at the end (ratio).
    pub grad_end: f64,
    /// Explicit total height change of a `[spline_h]` record (m), if any.
    pub rise: Option<f64>,
}

impl Elevation {
    /// Height after `s` metres of a lane of `length` metres.
    #[must_use]
    pub fn at(&self, s: f64, length: f64) -> f64 {
        if length <= 1e-9 {
            return self.z0;
        }
        self.z0
            + opts_core::formats::map::spline_height_offset(
                self.grad_start,
                self.grad_end,
                self.rise,
                s,
                length,
            )
    }

    /// The same profile walked backwards.
    #[must_use]
    pub fn reversed(&self, length: f64) -> Self {
        Self {
            z0: self.at(length, length),
            grad_start: -self.grad_end,
            grad_end: -self.grad_start,
            rise: self.rise.map(|h| -h),
        }
    }
}

/// A path in world space as read from the map, before direction expansion.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneSegment {
    /// Geometry in world space.
    pub geometry: PathGeometry,
    /// Which traffic may use it.
    pub kind: LaneKind,
    /// Lane width in metres.
    pub width: f64,
    /// Allowed direction.
    pub direction: Direction,
    /// Speed limit in m/s from a `speedlimit` `[rule]`, if any.
    pub speed_limit: Option<f64>,
    /// The other `[rule]`s of the path.
    pub rules: LaneRules,
    /// Height profile.
    pub elevation: Elevation,
    /// The light that governs entering this lane (`[use_traffic_light]`), if any.
    pub signal: Option<crate::signals::SignalKey>,
    /// The path belongs to a scenery object (junction, crossing): vehicles do not change lanes
    /// on it.
    pub junction: bool,
    /// The indicator a crossing path states (`.sco` `[path]` parameter 12: `0` none, `2` left,
    /// `3` right); `None` when it states none (splines, value `1`) and the turn heuristic applies.
    pub indicator: Option<crate::agent::Blinker>,
    /// Opaque tag for the host (object or spline id) so lanes can be traced back.
    pub source: u64,
}

impl LaneSegment {
    /// A forward-only straight road segment (handy for tests and tools).
    #[must_use]
    pub fn straight(start: DVec2, heading: f64, length: f64, kind: LaneKind) -> Self {
        Self {
            geometry: PathGeometry::straight(start, heading, length),
            kind,
            width: 2.5,
            direction: Direction::Forward,
            speed_limit: None,
            rules: LaneRules::default(),
            elevation: Elevation::default(),
            signal: None,
            junction: false,
            indicator: None,
            source: 0,
        }
    }
}

/// A directed lane.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    /// Geometry in travel direction.
    pub geometry: PathGeometry,
    /// Which traffic may use it.
    pub kind: LaneKind,
    /// Width in metres.
    pub width: f64,
    /// Speed limit in m/s, if any.
    pub speed_limit: Option<f64>,
    /// The other `[rule]`s of the path.
    pub rules: LaneRules,
    /// Height profile in travel direction.
    pub elevation: Elevation,
    /// The light that governs entering this lane.
    pub signal: Option<crate::signals::SignalKey>,
    /// The path belongs to a scenery object (see [`LaneSegment::junction`]).
    pub junction: bool,
    /// Indicator stated by the path (see [`LaneSegment::indicator`]).
    pub indicator: Option<crate::agent::Blinker>,
    /// The host's tag from the segment.
    pub source: u64,
    /// Stable identity across rebuilds: the segment's `source` and the number of lanes of that
    /// source before this one (reversed twins of `Both` lanes count).
    pub key: (u64, u32),
}

impl Lane {
    /// Length in metres.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.geometry.length
    }

    /// Height after `s` metres.
    #[must_use]
    pub fn z_at(&self, s: f64) -> f64 {
        self.elevation.at(s, self.length())
    }

    /// Heading change from start to end in radians (positive = clockwise = a right turn).
    #[must_use]
    pub fn turn(&self) -> f64 {
        let d = self.geometry.end_heading() - self.geometry.heading;
        (d + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
    }
}

/// Tolerances for joining lane ends to lane starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectParams {
    /// Largest distance between an end and a start, metres.
    pub position_tolerance: f64,
    /// Largest heading difference, radians.
    pub heading_tolerance: f64,
}

impl Default for ConnectParams {
    /// 0.75 m and 15°: wider than the ≤ 5 cm gaps of well-built splines, tight enough not to
    /// join neighbouring lanes (≥ 2.5 m apart) ❓ not yet calibrated against OMSI.
    fn default() -> Self {
        Self {
            position_tolerance: 0.75,
            heading_tolerance: 15f64.to_radians(),
        }
    }
}

/// Failure to place an agent.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpawnError {
    /// The lane id does not exist.
    #[error("no such lane")]
    NoSuchLane,
    /// Another agent is too close to the requested place.
    #[error("not enough free room at the requested position")]
    Occupied,
}

/// Directed lanes with their successors.
#[derive(Debug, Clone, Default)]
pub struct LaneGraph {
    lanes: Vec<Lane>,
    successors: Vec<Vec<LaneId>>,
    predecessors: Vec<Vec<LaneId>>,
    /// Spatial hash of lane samples for conflict search.
    pub(crate) grid: HashMap<(i64, i64), Vec<u32>>,
    /// Conflict zones per lane, computed on first use.
    pub(crate) conflict_cache: Vec<OnceLock<Vec<crate::conflict::Conflict>>>,
    /// Parallel same-direction lanes per lane, computed on first use.
    pub(crate) neighbour_cache: Vec<OnceLock<Vec<crate::neighbours::Neighbour>>>,
}

fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(std::f64::consts::TAU);
    d.min(std::f64::consts::TAU - d)
}

impl LaneGraph {
    /// Builds the graph: expands directions, then joins ends to starts of the same kind.
    #[must_use]
    pub fn build(segments: &[LaneSegment], params: &ConnectParams) -> Self {
        let mut lanes = Vec::new();
        let mut per_source: HashMap<u64, u32> = HashMap::new();
        for s in segments {
            let mut mk = |geometry: PathGeometry, elevation: Elevation, reversed: bool| {
                let n = per_source.entry(s.source).or_default();
                let key = (s.source, *n);
                *n += 1;
                Lane {
                    geometry,
                    kind: s.kind,
                    width: s.width,
                    speed_limit: s.speed_limit,
                    rules: s.rules,
                    elevation,
                    signal: s.signal,
                    junction: s.junction,
                    // Driven backwards, a turn indicator would point the wrong way.
                    indicator: if reversed { None } else { s.indicator },
                    source: s.source,
                    key,
                }
            };
            let back = s.elevation.reversed(s.geometry.length);
            match s.direction {
                Direction::Forward => lanes.push(mk(s.geometry, s.elevation, false)),
                Direction::Reverse => lanes.push(mk(s.geometry.reversed(), back, true)),
                Direction::Both => {
                    lanes.push(mk(s.geometry, s.elevation, false));
                    lanes.push(mk(s.geometry.reversed(), back, true));
                }
            }
        }
        // Spatial hash of lane starts.
        let cell = params.position_tolerance.max(0.01) * 2.0;
        let key = |p: DVec2| ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64);
        let mut starts: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (i, l) in lanes.iter().enumerate() {
            starts
                .entry(key(l.geometry.start))
                .or_default()
                .push(i as u32);
        }
        let mut successors = vec![Vec::new(); lanes.len()];
        for (i, a) in lanes.iter().enumerate() {
            let (end, heading) = (a.geometry.end(), a.geometry.end_heading());
            let (cx, cy) = key(end);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for &j in starts.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                        let b = &lanes[j as usize];
                        // A lane never continues into its own reverse twin (a U-turn on the spot).
                        if j as usize == i
                            || b.kind != a.kind
                            || b.geometry.start.distance(end) > params.position_tolerance
                            || angle_diff(b.geometry.heading, heading) > params.heading_tolerance
                        {
                            continue;
                        }
                        successors[i].push(LaneId(j));
                    }
                }
            }
            successors[i].sort_unstable();
        }
        let mut predecessors = vec![Vec::new(); lanes.len()];
        for (i, succ) in successors.iter().enumerate() {
            for s in succ {
                predecessors[s.0 as usize].push(LaneId(i as u32));
            }
        }
        let grid = Self::index_lanes(&lanes);
        let conflict_cache = (0..lanes.len()).map(|_| OnceLock::new()).collect();
        let neighbour_cache = (0..lanes.len()).map(|_| OnceLock::new()).collect();
        Self {
            lanes,
            successors,
            predecessors,
            grid,
            conflict_cache,
            neighbour_cache,
        }
    }

    /// Number of lanes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lanes.len()
    }

    /// Whether the graph has no lanes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }

    /// All lane ids.
    pub fn lane_ids(&self) -> impl Iterator<Item = LaneId> {
        (0..self.lanes.len() as u32).map(LaneId)
    }

    /// The lane `id`.
    #[must_use]
    pub fn lane(&self, id: LaneId) -> Option<&Lane> {
        self.lanes.get(id.0 as usize)
    }

    /// Lanes that continue `id`, sorted by id.
    #[must_use]
    pub fn successors(&self, id: LaneId) -> &[LaneId] {
        self.successors
            .get(id.0 as usize)
            .map_or(&[], Vec::as_slice)
    }

    /// The lane with stable key `key` (see [`Lane::key`]).
    #[must_use]
    pub fn lane_by_key(&self, key: (u64, u32)) -> Option<LaneId> {
        self.lanes
            .iter()
            .position(|l| l.key == key)
            .map(|i| LaneId(i as u32))
    }

    /// The place on an accepted lane nearest to `point` among those that run within
    /// `max_heading_diff` radians of `heading` (the direction a vehicle standing there faces):
    /// the lane, the distance along it and the distance from `point` (m). Found by sampling the
    /// lane every metre; `None` when no such lane lies within `max_dist`.
    #[must_use]
    pub fn nearest_place(
        &self,
        point: DVec2,
        heading: f64,
        max_heading_diff: f64,
        max_dist: f64,
        accept: impl Fn(&Lane) -> bool,
    ) -> Option<(LaneId, f64, f64)> {
        let mut best: Option<(LaneId, f64, f64)> = None;
        for (i, lane) in self.lanes.iter().enumerate() {
            if chord_distance(lane, point) > max_dist + 0.25 * lane.length() || !accept(lane) {
                continue;
            }
            let len = lane.length();
            let mut s = 0.0;
            while s <= len + 1e-9 {
                let d = lane.geometry.point_at(s).distance(point);
                let mut diff =
                    (lane.geometry.heading_at(s) - heading).rem_euclid(std::f64::consts::TAU);
                if diff > std::f64::consts::PI {
                    diff = std::f64::consts::TAU - diff;
                }
                if d <= max_dist && diff <= max_heading_diff && best.is_none_or(|b| d < b.2) {
                    best = Some((LaneId(i as u32), s.min(len), d));
                }
                s += 1.0;
            }
        }
        best
    }

    /// A random place on an accepted lane at a distance of `min..=max` metres from `centre`
    /// (up to 12 tries). `accept` filters lanes, e.g. by kind and density rule.
    pub fn random_spot(
        &self,
        centre: DVec2,
        min: f64,
        max: f64,
        rng: &mut Rng,
        accept: impl Fn(&Lane) -> bool,
    ) -> Option<(LaneId, f64)> {
        // Lanes with a start or end within reach; lane lengths are short against the radius.
        let near: Vec<u32> = self
            .lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                // Distance to the chord, with slack for the bulge of an arc.
                chord_distance(l, centre) <= max + 0.25 * l.length() && accept(l)
            })
            .map(|(i, _)| i as u32)
            .collect();
        if near.is_empty() {
            return None;
        }
        for _ in 0..12 {
            let id = LaneId(near[rng.below(near.len())]);
            let lane = &self.lanes[id.0 as usize];
            let s = rng.next_f64() * lane.length();
            let d = lane.geometry.point_at(s).distance(centre);
            if (min..=max).contains(&d) {
                return Some((id, s));
            }
        }
        None
    }

    /// A random place on a lane at a distance of `min..=max` metres from `centre`, the lane drawn
    /// with probability proportional to `weight` (lanes weighing `0` or less are never drawn; up
    /// to 12 tries). Spawning uses `length × density` as the weight.
    pub fn weighted_spot(
        &self,
        centre: DVec2,
        min: f64,
        max: f64,
        rng: &mut Rng,
        weight: impl Fn(&Lane) -> f64,
    ) -> Option<(LaneId, f64)> {
        let mut near: Vec<(u32, f64)> = Vec::new();
        let mut total = 0.0;
        for (i, l) in self.lanes.iter().enumerate() {
            if chord_distance(l, centre) > max + 0.25 * l.length() {
                continue;
            }
            let w = weight(l);
            if w > 0.0 && w.is_finite() {
                total += w;
                near.push((i as u32, total));
            }
        }
        if near.is_empty() {
            return None;
        }
        for _ in 0..12 {
            let pick = rng.next_f64() * total;
            let at = near
                .partition_point(|&(_, acc)| acc <= pick)
                .min(near.len() - 1);
            let id = LaneId(near[at].0);
            let lane = &self.lanes[id.0 as usize];
            let s = rng.next_f64() * lane.length();
            let d = lane.geometry.point_at(s).distance(centre);
            if (min..=max).contains(&d) {
                return Some((id, s));
            }
        }
        None
    }

    /// Lanes that lead into `id`, sorted by id.
    #[must_use]
    pub fn predecessors(&self, id: LaneId) -> &[LaneId] {
        self.predecessors
            .get(id.0 as usize)
            .map_or(&[], Vec::as_slice)
    }

    /// Shortest route (by length) from `from` to `to`, both included.
    #[must_use]
    pub fn shortest_route(&self, from: LaneId, to: LaneId) -> Option<Vec<LaneId>> {
        #[derive(PartialEq)]
        struct Item(f64, u32);
        impl Eq for Item {}
        impl Ord for Item {
            fn cmp(&self, o: &Self) -> Ordering {
                o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
            }
        }
        impl PartialOrd for Item {
            fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
                Some(self.cmp(o))
            }
        }
        self.lane(from)?;
        self.lane(to)?;
        let mut dist = vec![f64::INFINITY; self.lanes.len()];
        let mut prev = vec![u32::MAX; self.lanes.len()];
        let mut heap = BinaryHeap::new();
        dist[from.0 as usize] = self.lanes[from.0 as usize].length();
        heap.push(Item(dist[from.0 as usize], from.0));
        while let Some(Item(d, u)) = heap.pop() {
            if u == to.0 {
                let mut route = vec![to];
                let mut cur = to.0;
                while prev[cur as usize] != u32::MAX {
                    cur = prev[cur as usize];
                    route.push(LaneId(cur));
                }
                route.reverse();
                return Some(route);
            }
            if d > dist[u as usize] {
                continue;
            }
            for &LaneId(v) in &self.successors[u as usize] {
                let nd = d + self.lanes[v as usize].length();
                if nd < dist[v as usize] {
                    dist[v as usize] = nd;
                    prev[v as usize] = u;
                    heap.push(Item(nd, v));
                }
            }
        }
        None
    }
}

/// Distance from `centre` to the chord between a lane's ends.
fn chord_distance(l: &Lane, centre: DVec2) -> f64 {
    let (a, b) = (l.geometry.start, l.geometry.end());
    let ab = b - a;
    let t = if ab.length_squared() > 1e-9 {
        ((centre - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(centre)
}

#[cfg(test)]
mod tests;
