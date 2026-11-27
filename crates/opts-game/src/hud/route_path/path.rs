//! The route path resource: the stops of the route and the road pieces joining them.

use super::*;

/// A stop of the run with its position and the lane it lies on, if that is resident.
#[derive(Debug, Clone, Copy)]
pub struct RouteStop {
    /// World position (east, north).
    pub pos: DVec2,
    /// Lane and distance along it.
    pub place: Option<(LaneId, f64)>,
}

/// What the maps draw of the player's route.
#[derive(Resource, Default, Debug, Clone)]
pub struct PlayerRoutePath {
    /// The path as road pieces in the world frame.
    pub roads: Vec<MapRoad>,
    /// The stops ahead; the flag marks the next one.
    pub stops: Vec<(DVec2, bool)>,
    /// The lanes of the selected timetable trip (the course, M-0105), as resolved on the lanes
    /// that are resident; empty without a trip or without timetables.
    pub course: Vec<MapRoad>,
    /// Counted up when the route changes (so a map redraws).
    pub generation: u64,
    pub(super) known: HashMap<u64, (DVec2, f64)>,
    pub(super) since: f32,
}

impl PlayerRoutePath {
    /// Whether there is a route to draw.
    #[must_use]
    pub fn active(&self) -> bool {
        !self.stops.is_empty()
    }
}

/// The road pieces from stop to stop: the lanes between two resident places, a straight
/// connector otherwise.
#[must_use]
pub fn path_between(graph: &LaneGraph, stops: &[RouteStop]) -> Vec<MapRoad> {
    let mut out = Vec::new();
    for pair in stops.windows(2) {
        let lanes = match (pair[0].place, pair[1].place) {
            (Some(a), Some(b)) if a.0 == b.0 && b.1 >= a.1 => Some(vec![a.0]),
            (Some(a), Some(b)) => graph.shortest_route(a.0, b.0),
            _ => None,
        };
        match lanes {
            Some(lanes) => {
                out.extend(
                    lanes
                        .iter()
                        .filter_map(|l| graph.lane(*l))
                        .map(|l| MapRoad {
                            geometry: l.geometry,
                            width: PATH_WIDTH_M,
                            on_course: false,
                            ahead: false,
                        }),
                )
            }
            None => {
                let d = pair[1].pos - pair[0].pos;
                out.push(MapRoad {
                    geometry: PathGeometry::straight(pair[0].pos, d.x.atan2(d.y), d.length()),
                    width: PATH_WIDTH_M,
                    on_course: false,
                    ahead: false,
                });
            }
        }
    }
    out
}
