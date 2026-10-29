//! Turns placed splines and objects of a map tile into [`LaneSegment`]s.
//!
//! * A **spline** carries its lanes in its `.sli` profile (`[path]`: type, lateral offset,
//!   width, direction); the lane follows the spline's centre line shifted sideways by the offset
//!   (negated on a `mirror`ed spline, like the mesh). Spline lanes arc for every radius and take
//!   the height profile of the mesh (`[spline_h]` ramp).
//! * A **scenery object** (junction, crossing, bus stop) carries object-local `[path]` records in
//!   its `.sco`; they are rotated clockwise by the object's heading and moved to its position.
//!
//! The host supplies the parsed `.sli`/`.sco` through [`PathSource`] (it already caches them);
//! this module does no I/O. The tile's `[rule]` records are applied to the lane they address (owner
//! element and path index in file order; speed limits are km/h in the file). Splines and paths
//! whose length is not a positive number are skipped (the installed maps contain a few with
//! negative lengths such as `-9.11`; what they mean is an open question). Spline arms embedded in
//! objects with `[splinehelper]` are not converted yet: rule indices of such objects continue
//! into the helpers' paths (`docs/src/formats/map/tiles.md`).

use glam::DVec2;
use opts_core::formats::map::{
    MapObjectInstance, MapTile, MapTrafficRule, RuleKind, RuleOwner, TileGrid,
};
use opts_core::formats::scenery::{ScoPath, ScoTrafficLightGroup, SplinePath};

use crate::agent::Blinker;
use crate::geometry::{PathGeometry, SCO_STRAIGHT_RADIUS};
use crate::graph::{Direction, Elevation, LaneKind, LaneRules, LaneSegment};
use crate::signals::{SignalKey, SignalProgram};

/// World origin of tile `(x, y)` on the map's tile grid, relative to tile `(0, 0)` (on a 300 m
/// grid `(x · 300, y · 300)`; `docs/src/architecture/streaming-coordinates.md`).
#[must_use]
pub fn tile_origin(grid: &TileGrid, x: i32, y: i32) -> DVec2 {
    grid.origin((x, y), (0, 0))
}

/// Lookup of parsed path data by the file name written in the tile.
pub trait PathSource {
    /// The `[path]` records of a `.sli` profile; `None` when the file is unknown or unreadable.
    fn sli_paths(&mut self, sli_file: &str) -> Option<&[SplinePath]>;
    /// The `[path]` records of a `.sco`; `None` when the file is unknown or unreadable.
    fn sco_paths(&mut self, sco_file: &str) -> Option<&[ScoPath]>;
    /// Height of the object's origin in metres. The default is the position's `z` as written in
    /// the tile; hosts add the terrain height for objects that are not stored at absolute height
    /// (`[absheight]`, crossings with spline helpers).
    fn object_base_height(&mut self, object: &MapObjectInstance) -> f64 {
        f64::from(object.position.z)
    }
    /// The light programs of a `.sco` (`None` when the file is unknown).
    fn sco_lights(&mut self, _sco_file: &str) -> Option<&[ScoTrafficLightGroup]> {
        None
    }
}

/// The light programs of every object of `tile`, keyed like [`LaneSegment::signal`]: the object's
/// `id_code` and the program's index in declaration order across its groups.
pub fn signals_from_tile(
    tile: &MapTile,
    source: &mut impl PathSource,
) -> Vec<(SignalKey, SignalProgram)> {
    let mut out = Vec::new();
    for o in &tile.objects {
        let Some(groups) = source.sco_lights(&o.sco_file) else {
            continue;
        };
        let programs = groups.iter().flat_map(|g| {
            g.programs
                .iter()
                .map(move |p| (f64::from(g.period_sec()), p))
        });
        for (i, (period, p)) in programs.enumerate() {
            let phases = p
                .phases
                .iter()
                .map(|ph| (ph.state, f64::from(ph.duration_sec)))
                .collect();
            let approach = f64::from(p.approach_distance_m.unwrap_or(0.0));
            if let Some(program) = SignalProgram::in_period(phases, period, approach) {
                out.push(((o.id_code, i), program));
            }
        }
    }
    out
}

/// `speedlimit` values (km/h) from this on mean "no limit".
pub const NO_LIMIT_KMH: f64 = 200.0;

/// Folds the rules that address path `path_index` of `owner` into a speed limit (m/s) and the
/// other lane rules.
fn rules_for(
    rules: &[MapTrafficRule],
    owner: RuleOwner,
    path_index: usize,
) -> (Option<f64>, LaneRules) {
    let mut speed = None;
    let mut out = LaneRules::default();
    for r in rules
        .iter()
        .filter(|r| r.owner == Some(owner) && r.path_index == path_index)
    {
        let value = f64::from(r.value);
        match r.kind() {
            // km/h in the file; `0` would stop all traffic, so it counts as "no limit" ❓, and so
            // do 200 km/h and more (the editor's "no limit" is 250, Q-AK-02).
            Some(RuleKind::SpeedLimit) if value > 0.0 && value < NO_LIMIT_KMH => {
                speed = Some(value / 3.6);
            }
            // The flag names the group; the last rule for a group wins (file order).
            Some(RuleKind::TrafficDensity) => {
                out.density.set(r.flag as usize, value);
            }
            Some(RuleKind::Priority) => out.priority = Some(r.value.max(0.0) as u32),
            Some(RuleKind::Trucks) => out.trucks = true,
            Some(RuleKind::Bus) => out.bus = true,
            Some(RuleKind::NoCars) => out.no_cars = true,
            Some(RuleKind::OvertakingProhibited) => out.no_overtaking = true,
            _ => {}
        }
    }
    (speed, out)
}

fn spline_segments(
    tile: &MapTile,
    origin: DVec2,
    source: &mut impl PathSource,
) -> Vec<LaneSegment> {
    let mut out = Vec::new();
    for (si, s) in tile.splines.iter().enumerate() {
        if !(s.length_m.is_finite() && s.length_m > 0.0) {
            continue;
        }
        let Some(paths) = source.sli_paths(&s.sli_file) else {
            continue;
        };
        let centre = PathGeometry {
            start: origin + DVec2::new(f64::from(s.position.x), f64::from(s.position.y)),
            heading: f64::from(s.heading_deg).to_radians(),
            radius: f64::from(s.radius_m),
            length: f64::from(s.length_m),
        };
        let owner = RuleOwner::Spline {
            index: si,
            id_code: s.id_code,
        };
        let mirror = if s.mirrored { -1.0 } else { 1.0 };
        for (pi, p) in paths.iter().enumerate() {
            let Some(kind) = LaneKind::from_path_type(p.path_type) else {
                continue;
            };
            let (speed_limit, rules) = rules_for(&tile.rules, owner, pi);
            let geometry = centre.offset(mirror * f64::from(p.offset_x));
            let scale = centre.length / geometry.length.max(1e-6);
            out.push(LaneSegment {
                geometry,
                kind,
                width: f64::from(p.width),
                direction: if s.mirrored {
                    Direction::from_number(p.direction).reversed()
                } else {
                    Direction::from_number(p.direction)
                },
                speed_limit,
                rules,
                elevation: Elevation {
                    z0: f64::from(s.position.z) + f64::from(p.offset_z),
                    grad_start: f64::from(s.gradient_start) * scale,
                    grad_end: f64::from(s.gradient_end) * scale,
                    rise: s.explicit_rise().map(f64::from),
                },
                signal: None,
                junction: false,
                indicator: None,
                source: s.id_code,
            });
        }
    }
    out
}

fn object_segments(
    tile: &MapTile,
    origin: DVec2,
    source: &mut impl PathSource,
) -> Vec<LaneSegment> {
    let mut out = Vec::new();
    for (oi, o) in tile.objects.iter().enumerate() {
        if source
            .sco_paths(&o.sco_file)
            .is_none_or(<[ScoPath]>::is_empty)
        {
            continue;
        }
        let base_height = source.object_base_height(o);
        let Some(paths) = source.sco_paths(&o.sco_file) else {
            continue;
        };
        let position = origin + DVec2::new(f64::from(o.position.x), f64::from(o.position.y));
        let heading = f64::from(o.heading_deg).to_radians();
        let owner = RuleOwner::Object {
            index: oi,
            id_code: o.id_code,
        };
        for (pi, p) in paths.iter().enumerate() {
            let Some(kind) = LaneKind::from_path_type(p.path_type) else {
                continue;
            };
            if !(p.length_m.is_finite() && p.length_m > 0.0) {
                continue;
            }
            let local = PathGeometry {
                start: DVec2::new(f64::from(p.start.x), f64::from(p.start.y)),
                heading: f64::from(p.heading_deg).to_radians(),
                radius: match f64::from(p.radius_m) {
                    r if r.abs() >= SCO_STRAIGHT_RADIUS => 0.0,
                    r => r,
                },
                length: f64::from(p.length_m),
            };
            let (speed_limit, rules) = rules_for(&tile.rules, owner, pi);
            out.push(LaneSegment {
                geometry: local.to_world(position, heading),
                kind,
                width: f64::from(p.width_m),
                direction: Direction::from_number(p.direction),
                speed_limit,
                rules,
                elevation: Elevation {
                    z0: base_height + f64::from(p.start.z),
                    grad_start: f64::from(p.slope_start),
                    grad_end: f64::from(p.slope_end),
                    rise: None,
                },
                signal: p.traffic_light_index.map(|i| (o.id_code, i)),
                junction: true,
                indicator: match p.extra {
                    Some(0) => Some(Blinker::Off),
                    Some(2) => Some(Blinker::Left),
                    Some(3) => Some(Blinker::Right),
                    _ => None,
                },
                source: o.id_code,
            });
        }
    }
    out
}

/// Lane segments of every spline and object of `tile`, which sits at `origin`.
///
/// Pedestrian and rail paths are included (see [`LaneKind`]); filter by kind when building a
/// road-only graph.
pub fn segments_from_tile(
    tile: &MapTile,
    origin: DVec2,
    source: &mut impl PathSource,
) -> Vec<LaneSegment> {
    let mut out = spline_segments(tile, origin, source);
    out.extend(object_segments(tile, origin, source));
    out
}

#[cfg(test)]
mod tests;
