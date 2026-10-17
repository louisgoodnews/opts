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
/// The vehicle-type flags read as their names say (project decision, M-0049): `bus` makes the lane
/// a lane **for buses**, `trucks` one **for trucks**, `no_cars` keeps **cars** off it. The corpus
/// has them only with value `0`; the semantics are not confirmed in OMSI ❓.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaneRules {
    /// `trafficdensity`: factor for AI traffic on the lane (`0` = none; corpus values `0`,
    /// `0.001`, `0.1`, `0.5`, `3`, …).
    pub density: Option<f64>,
    /// `priority`: raw value (corpus: `64` and `192`; the order is not settled ❓).
    pub priority: Option<u32>,
    /// A `trucks` rule exists.
    pub trucks: bool,
    /// A `bus` rule exists.
    pub bus: bool,
    /// A `no_cars` rule exists.
    pub no_cars: bool,
    /// An `overtaking_prohib` rule exists.
    pub no_overtaking: bool,
}

impl LaneRules {
    /// Whether vehicles of `class` may use the lane: `bus` and `trucks` restrict the lane to the
    /// classes they name (both together admit both), `no_cars` excludes cars.
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
            let mut mk = |geometry: PathGeometry, elevation: Elevation| {
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
                    source: s.source,
                    key,
                }
            };
            let back = s.elevation.reversed(s.geometry.length);
            match s.direction {
                Direction::Forward => lanes.push(mk(s.geometry, s.elevation)),
                Direction::Reverse => lanes.push(mk(s.geometry.reversed(), back)),
                Direction::Both => {
                    lanes.push(mk(s.geometry, s.elevation));
                    lanes.push(mk(s.geometry.reversed(), back));
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
                let (a, b) = (l.geometry.start, l.geometry.end());
                let ab = b - a;
                let t = if ab.length_squared() > 1e-9 {
                    ((centre - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (a + ab * t).distance(centre) <= max + 0.25 * l.length() && accept(l)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn road(x: f64, y: f64, heading: f64, len: f64) -> LaneSegment {
        LaneSegment::straight(DVec2::new(x, y), heading, len, LaneKind::Road)
    }

    #[test]
    fn end_to_start_joins_and_gaps_do_not() {
        let g = LaneGraph::build(
            &[
                road(0.0, 0.0, 0.0, 50.0),
                road(0.0, 50.2, 0.0, 50.0),
                road(0.0, 120.0, 0.0, 10.0),
            ],
            &ConnectParams::default(),
        );
        assert_eq!(g.successors(LaneId(0)), &[LaneId(1)]);
        assert!(g.successors(LaneId(1)).is_empty());
    }

    #[test]
    fn heading_must_match() {
        // Second lane starts at the right place but points east.
        let g = LaneGraph::build(
            &[
                road(0.0, 0.0, 0.0, 50.0),
                road(0.0, 50.0, std::f64::consts::FRAC_PI_2, 20.0),
            ],
            &ConnectParams::default(),
        );
        assert!(g.successors(LaneId(0)).is_empty());
    }

    #[test]
    fn kinds_do_not_mix() {
        let mut ped = road(0.0, 50.0, 0.0, 10.0);
        ped.kind = LaneKind::Pedestrian;
        let g = LaneGraph::build(&[road(0.0, 0.0, 0.0, 50.0), ped], &ConnectParams::default());
        assert!(g.successors(LaneId(0)).is_empty());
    }

    #[test]
    fn directions_expand_to_directed_lanes() {
        let mut both = road(0.0, 0.0, 0.0, 30.0);
        both.direction = Direction::Both;
        let mut rev = road(0.0, 100.0, 0.0, 30.0);
        rev.direction = Direction::Reverse;
        let g = LaneGraph::build(&[both, rev], &ConnectParams::default());
        assert_eq!(g.len(), 3);
        // The reversed lane starts at the original end and heads south.
        let l = g.lane(LaneId(2)).unwrap();
        assert!(l.geometry.start.abs_diff_eq(DVec2::new(0.0, 130.0), 1e-9));
        assert!(angle_diff(l.geometry.heading, std::f64::consts::PI) < 1e-9);
    }

    #[test]
    fn vehicle_type_rules_read_as_their_names_say() {
        use crate::idm::VehicleClass::{Bus, Car, Truck};
        let none = LaneRules::default();
        assert!([Car, Truck, Bus].iter().all(|c| none.allows(*c)));
        let bus_lane = LaneRules { bus: true, ..none };
        assert_eq!(
            [
                bus_lane.allows(Car),
                bus_lane.allows(Truck),
                bus_lane.allows(Bus)
            ],
            [false, false, true]
        );
        let truck_lane = LaneRules {
            trucks: true,
            ..none
        };
        assert_eq!(
            [
                truck_lane.allows(Car),
                truck_lane.allows(Truck),
                truck_lane.allows(Bus)
            ],
            [false, true, false]
        );
        let both = LaneRules {
            bus: true,
            trucks: true,
            ..none
        };
        assert_eq!(
            [both.allows(Car), both.allows(Truck), both.allows(Bus)],
            [false, true, true]
        );
        let no_cars = LaneRules {
            no_cars: true,
            ..none
        };
        assert_eq!(
            [
                no_cars.allows(Car),
                no_cars.allows(Truck),
                no_cars.allows(Bus)
            ],
            [false, true, true]
        );
    }

    #[test]
    fn elevation_follows_the_gradients_and_reverses() {
        let e = Elevation {
            z0: 10.0,
            grad_start: 0.02,
            grad_end: 0.06,
            rise: None,
        };
        // rise = length · (g0 + g1) / 2 = 100 · 0.04 = 4 m.
        assert!((e.at(100.0, 100.0) - 14.0).abs() < 1e-9);
        assert!((e.at(50.0, 100.0) - (10.0 + 50.0 * 0.03)).abs() < 1e-9);
        let back = e.reversed(100.0);
        assert!((back.z0 - 14.0).abs() < 1e-9);
        assert!((back.at(100.0, 100.0) - 10.0).abs() < 1e-9);
        assert!((back.at(50.0, 100.0) - e.at(50.0, 100.0)).abs() < 1e-9);
    }

    #[test]
    fn spline_h_elevation_is_the_mesh_ramp_and_reverses() {
        // Flat ends, 2 m up over 40 m: the S-shaped ramp of the mesh, not a linear gradient.
        let e = Elevation {
            z0: 5.0,
            grad_start: 0.0,
            grad_end: 0.0,
            rise: Some(2.0),
        };
        assert!((e.at(40.0, 40.0) - 7.0).abs() < 1e-9);
        assert!((e.at(20.0, 40.0) - 6.0).abs() < 1e-9);
        assert!(e.at(4.0, 40.0) - 5.0 < 0.06, "starts flat");
        let back = e.reversed(40.0);
        for s in [0.0, 7.0, 20.0, 33.0, 40.0] {
            assert!((back.at(40.0 - s, 40.0) - e.at(s, 40.0)).abs() < 1e-9);
        }
    }

    #[test]
    fn keys_survive_a_rebuild_with_other_segments() {
        let mut a = road(0.0, 0.0, 0.0, 30.0);
        a.source = 5;
        a.direction = Direction::Both;
        let mut b = road(0.0, 100.0, 0.0, 30.0);
        b.source = 6;
        let g1 = LaneGraph::build(&[a.clone(), b.clone()], &ConnectParams::default());
        let g2 = LaneGraph::build(&[b, a], &ConnectParams::default());
        assert_eq!(g1.lane_by_key((5, 1)), Some(LaneId(1)));
        assert_eq!(g2.lane_by_key((5, 1)), Some(LaneId(2)));
        assert_eq!(g2.lane_by_key((9, 0)), None);
    }

    #[test]
    fn random_spots_lie_in_the_requested_band() {
        let g = LaneGraph::build(&[road(0.0, 0.0, 0.0, 400.0)], &ConnectParams::default());
        let mut rng = Rng::new(9);
        let mut found = 0;
        for _ in 0..50 {
            if let Some((id, s)) =
                g.random_spot(DVec2::new(0.0, 200.0), 50.0, 120.0, &mut rng, |_| true)
            {
                let d = g
                    .lane(id)
                    .unwrap()
                    .geometry
                    .point_at(s)
                    .distance(DVec2::new(0.0, 200.0));
                assert!((50.0..=120.0).contains(&d));
                found += 1;
            }
        }
        assert!(found > 20, "only {found} spots");
        assert!(g
            .random_spot(DVec2::new(0.0, 200.0), 50.0, 120.0, &mut rng, |_| false)
            .is_none());
    }

    #[test]
    fn shortest_route_picks_the_shorter_branch() {
        // A fork after lane 0: lane 1 is long, lane 2 short, both rejoin in lane 3 (straight
        // stubs are enough to test the search).
        let mut long = road(0.0, 50.0, 0.0, 200.0);
        long.source = 1;
        let short = road(0.0, 50.0, 0.0, 20.0);
        let end_long = road(0.0, 250.0, 0.0, 10.0);
        let end_short = road(0.0, 70.0, 0.0, 10.0);
        let g = LaneGraph::build(
            &[road(0.0, 0.0, 0.0, 50.0), long, short, end_long, end_short],
            &ConnectParams::default(),
        );
        assert_eq!(g.successors(LaneId(0)).len(), 2);
        let route = g.shortest_route(LaneId(0), LaneId(4)).unwrap();
        assert_eq!(route, [LaneId(0), LaneId(2), LaneId(4)]);
        assert!(g.shortest_route(LaneId(4), LaneId(0)).is_none());
    }
}
