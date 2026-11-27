//! Builds the route path each rebuild: the stops ahead, their lanes and the path between them.

use super::*;

/// The stop index and station ids of the player's schedule, if it has one.
pub(super) fn scheduled_stops(
    player: &Query<&VehicleRoute, With<PlayerVehicle>>,
) -> Option<(usize, Vec<u32>)> {
    player.iter().find_map(|r| {
        r.handle.with(|s| {
            s.schedule.as_ref().map(|sc| {
                let ids: Vec<u32> = sc.stops.iter().map(|x| x.station_id).collect();
                (s.stop_index, ids)
            })
        })
    })
}

/// The lanes of the player's timetable trip as road pieces (M-0105 course). The trip comes from
/// the route's timetables and schedule; its entries are looked up on the resident lane graph, so
/// entries on tiles that are not resident are left out here (they are reported by the resolver).
pub(super) fn course_roads(
    player: &Query<&VehicleRoute, With<PlayerVehicle>>,
    graph: Option<&opts_traffic::LaneGraph>,
) -> (Vec<LaneId>, Vec<MapRoad>) {
    let Some(graph) = graph else {
        return (Vec::new(), Vec::new());
    };
    let draft = player.iter().find_map(|r| {
        r.handle.with(|s| {
            let schedule = s.schedule.as_ref()?;
            let timetables = s.timetables.as_ref()?;
            let trip = timetables.trip(&schedule.trip)?;
            Some(opts_traffic::course_from_trip(timetables, trip))
        })
    });
    let Some(draft) = draft else {
        return (Vec::new(), Vec::new());
    };
    // Lane ids and road pieces are pushed together, so `course_lanes[k]` is the lane of
    // `course[k]` (the mini map's flags and the matcher index the course by that order).
    graph
        .resolve_course(&draft.lanes)
        .resolved
        .iter()
        .filter_map(|r| {
            graph.lane(r.lane).map(|lane| {
                (
                    r.lane,
                    MapRoad {
                        geometry: lane.geometry,
                        width: lane.width,
                        on_course: true,
                        ahead: false,
                    },
                )
            })
        })
        .unzip()
}

/// The stop markers in the world give the exact position and heading of their stops.
pub(super) fn remember_markers(
    path: &mut PlayerRoutePath,
    markers: &Query<(&BusStopMarker, &GlobalTransform)>,
) {
    for (m, gt) in markers {
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
}

/// The known stops among `ahead`, each with its place on the lane graph.
pub(super) fn route_stops(
    path: &PlayerRoutePath,
    ahead: &[&u32],
    graph: Option<&opts_traffic::LaneGraph>,
) -> Vec<RouteStop> {
    ahead
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
        .collect()
}

pub(super) fn update_route_path(
    time: Res<Time<Real>>,
    mut path: ResMut<PlayerRoutePath>,
    player: Query<&VehicleRoute, With<PlayerVehicle>>,
    markers: Query<(&BusStopMarker, &GlobalTransform)>,
    traffic: Option<Res<TrafficState>>,
    mut overview: ResMut<super::super::fullmap::OverviewRoads>,
    job: Option<Res<crate::in_game::omsi_world::WorldLoadJob>>,
) {
    path.since += time.delta_secs();
    if path.since < REBUILD_S {
        return;
    }
    path.since = 0.0;
    let schedule = scheduled_stops(&player);
    let Some((index, ids)) = schedule else {
        if path.active() {
            path.roads.clear();
            path.stops.clear();
            path.course.clear();
            path.course_lanes.clear();
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
    remember_markers(&mut path, &markers);
    let graph = traffic.as_ref().map(|t| t.lane_graph());
    let ahead: Vec<&u32> = ids.iter().skip(index).take(STOPS_AHEAD).collect();
    let stops = route_stops(&path, &ahead, graph);
    let roads = graph.map_or_else(Vec::new, |g| path_between(g, &stops));
    let (course_lanes, course) = course_roads(&player, graph);
    let first_known = ahead
        .first()
        .is_some_and(|i| path.known.contains_key(&u64::from(**i)));
    let drawn: Vec<(DVec2, bool)> = stops
        .iter()
        .enumerate()
        .map(|(i, s)| (s.pos, i == 0 && first_known))
        .collect();
    if drawn != path.stops || roads.len() != path.roads.len() || course.len() != path.course.len()
    {
        path.generation += 1;
    }
    path.course = course;
    path.course_lanes = course_lanes;
    path.stops = drawn;
    path.roads = roads;
}
