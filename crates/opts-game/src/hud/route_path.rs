//! The player's followed route for the mini map and the large map: the road path from the
//! current stop of the timetable through the stops still ahead, and those stops.
//!
//! Stop positions come from the bus stop objects of the tiles seen so far (remembered when a tile
//! unloads, so the route grows as the drive goes on); the path between two stops follows the
//! lane graph where both are resident and is a straight connector where they are not.

use std::collections::HashMap;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::bus_stops::BusStopMarker;
use opts_bevy::route::VehicleRoute;
use opts_bevy::PlayerVehicle;
use opts_traffic::{LaneGraph, LaneId, LaneKind, PathGeometry};

use crate::hud::minimap::MapRoad;
use crate::in_game::omsi_world::TrafficState;

/// Stops ahead that are drawn.
const STOPS_AHEAD: usize = 14;
/// Seconds between two rebuilds of the path.
const REBUILD_S: f32 = 2.0;
/// Reach of a stop to a lane (m) and the heading tolerance, as for scheduled traffic.
const LANE_REACH_M: f64 = 9.0;
const LANE_HEADING_TOL: f64 = 0.65;
/// Width of the drawn path (m); the raster draws it at a fixed pixel width.
const PATH_WIDTH_M: f64 = 4.0;

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
    /// Counted up when the route changes (so a map redraws).
    pub generation: u64,
    known: HashMap<u64, (DVec2, f64)>,
    since: f32,
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
                        }),
                )
            }
            None => {
                let d = pair[1].pos - pair[0].pos;
                out.push(MapRoad {
                    geometry: PathGeometry::straight(pair[0].pos, d.x.atan2(d.y), d.length()),
                    width: PATH_WIDTH_M,
                });
            }
        }
    }
    out
}

#[allow(clippy::type_complexity)]
fn update_route_path(
    time: Res<Time<Real>>,
    mut path: ResMut<PlayerRoutePath>,
    player: Query<&VehicleRoute, With<PlayerVehicle>>,
    markers: Query<(&BusStopMarker, &GlobalTransform)>,
    traffic: Option<Res<TrafficState>>,
    mut overview: ResMut<super::fullmap::OverviewRoads>,
    job: Option<Res<crate::in_game::omsi_world::WorldLoadJob>>,
) {
    path.since += time.delta_secs();
    if path.since < REBUILD_S {
        return;
    }
    path.since = 0.0;
    let schedule = player.iter().find_map(|r| {
        r.handle.with(|s| {
            s.schedule.as_ref().map(|sc| {
                let ids: Vec<u32> = sc.stops.iter().map(|x| x.station_id).collect();
                (s.stop_index, ids)
            })
        })
    });
    let Some((index, ids)) = schedule else {
        if path.active() {
            path.roads.clear();
            path.stops.clear();
            path.generation += 1;
        }
        return;
    };
    // The stops of the whole map come from the reader of the large map's overview.
    if !overview.started() {
        if let Some(job) = &job {
            let (root, map_dir, tiles) = job.tile_listing();
            overview.start(root, map_dir, tiles);
        }
    }
    for id in ids.iter().skip(index).take(STOPS_AHEAD) {
        if let Some(found) = overview.stop(u64::from(*id)) {
            path.known.entry(u64::from(*id)).or_insert(found);
        }
    }
    for (m, gt) in &markers {
        let t = gt.translation();
        let f = gt.rotation() * Vec3::NEG_Z;
        path.known.insert(
            m.id_code,
            (
                DVec2::new(f64::from(t.x), -f64::from(t.z)),
                f64::from(f.x).atan2(-f64::from(f.z)),
            ),
        );
    }
    let graph = traffic.as_ref().map(|t| t.lane_graph());
    let ahead: Vec<&u32> = ids.iter().skip(index).take(STOPS_AHEAD).collect();
    let stops: Vec<RouteStop> = ahead
        .iter()
        .filter_map(|id| path.known.get(&u64::from(**id)))
        .map(|&(pos, heading)| RouteStop {
            pos,
            place: graph.and_then(|g| {
                g.nearest_place(pos, heading, LANE_HEADING_TOL, LANE_REACH_M, |l| {
                    l.kind == LaneKind::Road && l.length() > 4.0
                })
                .map(|(lane, s, _)| (lane, s))
            }),
        })
        .collect();
    let roads = graph.map_or_else(Vec::new, |g| path_between(g, &stops));
    let first_known = ahead
        .first()
        .is_some_and(|i| path.known.contains_key(&u64::from(**i)));
    let drawn: Vec<(DVec2, bool)> = stops
        .iter()
        .enumerate()
        .map(|(i, s)| (s.pos, i == 0 && first_known))
        .collect();
    if drawn != path.stops || roads.len() != path.roads.len() {
        path.generation += 1;
    }
    path.stops = drawn;
    path.roads = roads;
}

/// Where the scheduled vehicles are, for the maps.
#[derive(Resource, Default, Debug, Clone)]
pub struct ScheduledVehicles {
    /// World position (east, north) and compass heading.
    pub poses: Vec<(DVec2, f32)>,
    since: f32,
}

fn update_scheduled_vehicles(
    time: Res<Time<Real>>,
    mut out: ResMut<ScheduledVehicles>,
    traffic: Option<Res<TrafficState>>,
    sched: Option<Res<crate::in_game::omsi_world::ScheduledTraffic>>,
    trains: Query<&GlobalTransform, With<crate::in_game::omsi_world::ScheduledTrain>>,
) {
    out.since += time.delta_secs();
    if out.since < 0.2 {
        return;
    }
    out.since = 0.0;
    out.poses.clear();
    if let (Some(t), Some(s)) = (traffic, sched) {
        t.poses_where(|id| s.run_of(id).is_some(), &mut out.poses);
    }
    for gt in &trains {
        let (p, h) = crate::hud::minimap::pose_of(gt);
        out.poses.push((p, h));
    }
}

/// Registers the route path.
pub struct RoutePathPlugin;

impl Plugin for RoutePathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerRoutePath>()
            .init_resource::<ScheduledVehicles>()
            .add_systems(
                Update,
                (update_route_path, update_scheduled_vehicles)
                    .run_if(in_state(crate::app_state::AppState::InGame)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_lanes_the_stops_are_joined_by_straight_connectors() {
        let stops = [
            RouteStop {
                pos: DVec2::new(0.0, 0.0),
                place: None,
            },
            RouteStop {
                pos: DVec2::new(30.0, 40.0),
                place: None,
            },
        ];
        let roads = path_between(&LaneGraph::default(), &stops);
        assert_eq!(roads.len(), 1);
        assert!((roads[0].geometry.length - 50.0).abs() < 1e-9);
        let end = roads[0].geometry.point_at(50.0);
        assert!(end.distance(DVec2::new(30.0, 40.0)) < 1e-6);
    }
}
