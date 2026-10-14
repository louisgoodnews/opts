//! Where the mini map gets its roads from: a thin, read-only interface.
//!
//! The map never reads the tile code. A [`MapSource`] hands over the road paths near a point (the
//! lane graph of the traffic crate implements it, see `omsi_world::traffic`); fixtures implement
//! it for tests and the golden screenshot.

use bevy::math::DVec2;
use bevy::math::Vec2;
use bevy::prelude::Resource;
use opts_traffic::PathGeometry;

use super::raster::MapLine;

/// One road path in the world frame (x east, y north, metres; compass headings).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapRoad {
    /// Straight line or circular arc.
    pub geometry: PathGeometry,
    /// Width in metres.
    pub width: f64,
}

/// A read-only supplier of road paths.
pub trait MapSource {
    /// Pushes the paths that may touch the circle (`centre`, `radius_m`). Paths further away may
    /// be left out; a few extra ones are fine (the drawing clips).
    fn roads_near(&self, centre: DVec2, radius_m: f64, out: &mut Vec<MapRoad>);
}

/// Whether a path can touch the circle: its start is within `radius + length` of the centre.
#[must_use]
pub fn may_touch(road: &PathGeometry, centre: DVec2, radius_m: f64) -> bool {
    road.start.distance(centre) <= radius_m + road.length.max(0.0)
}

/// A fixed set of roads (tests and the golden screenshot; no map data).
#[derive(Resource, Debug, Clone, Default)]
pub struct FixtureRoads(pub Vec<MapRoad>);

impl MapSource for FixtureRoads {
    fn roads_near(&self, centre: DVec2, radius_m: f64, out: &mut Vec<MapRoad>) {
        out.extend(
            self.0
                .iter()
                .filter(|r| may_touch(&r.geometry, centre, radius_m)),
        );
    }
}

impl FixtureRoads {
    /// A self-authored street grid with one curve, around the origin.
    #[must_use]
    pub fn grid() -> Self {
        let mut roads = Vec::new();
        for i in -3..=3 {
            let o = f64::from(i) * 70.0;
            roads.push(MapRoad {
                geometry: PathGeometry::straight(
                    DVec2::new(-300.0, o),
                    std::f64::consts::FRAC_PI_2,
                    600.0,
                ),
                width: if i == 0 { 7.0 } else { 5.0 },
            });
            roads.push(MapRoad {
                geometry: PathGeometry::straight(DVec2::new(o, -300.0), 0.0, 600.0),
                width: if i == 0 { 7.0 } else { 5.0 },
            });
        }
        roads.push(MapRoad {
            geometry: PathGeometry {
                start: DVec2::new(-300.0, 250.0),
                heading: std::f64::consts::FRAC_PI_2,
                radius: -180.0,
                length: 420.0,
            },
            width: 6.0,
        });
        Self(roads)
    }
}

/// Cuts `road` into chords relative to `centre` and appends them to `out`. A chord is at least
/// `min_chord_m` long (arcs get fewer pieces when the map is zoomed out), so the number of pieces
/// follows the pixels, not the length of the road.
pub fn push_chords(road: &MapRoad, centre: DVec2, min_chord_m: f64, out: &mut Vec<MapLine>) {
    let g = &road.geometry;
    let rel = |p: DVec2| {
        let d = p - centre;
        Vec2::new(d.x as f32, d.y as f32)
    };
    let pieces = if g.is_straight() {
        1
    } else {
        ((g.length / min_chord_m.max(1.0)).ceil() as usize).clamp(1, 32)
    };
    let mut prev = rel(g.point_at(0.0));
    for i in 1..=pieces {
        let next = rel(g.point_at(g.length * i as f64 / pieces as f64));
        out.push(MapLine {
            a: prev,
            b: next,
            width_m: road.width as f32,
        });
        prev = next;
    }
}
