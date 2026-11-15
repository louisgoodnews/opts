//! The inputs of the full map's redraw: the markers, the roads and the route, in map pixels.

use super::super::*;
use super::thin_lines;

/// The player's and the session's vehicles as markers, or the free camera when there is no player.
pub(super) fn gather_markers(
    vehicles: &Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: &Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) -> Vec<(DVec2, f32, bool)> {
    let mut markers: Vec<(DVec2, f32, bool)> = Vec::new();
    for (t, session, is_player) in vehicles {
        if session.is_some() || is_player {
            let (pos, heading) = pose_of(t);
            markers.push((pos, heading, is_player));
        }
    }
    if !markers.iter().any(|m| m.2) {
        if let Some(t) = cameras.iter().next() {
            let (pos, heading) = pose_of(t);
            markers.push((pos, heading, true));
        }
    }
    markers
}

/// The roads around the centre: the lane graph's, the overview's for tiles that are not resident
/// and the fixture's (tests).
pub(super) fn collect_roads(
    centre: DVec2,
    radius_m: f64,
    (traffic, fixture): (Option<&TrafficState>, Option<&FixtureRoads>),
    overview: &OverviewRoads,
) -> Vec<crate::hud::minimap::MapRoad> {
    let mut roads = Vec::new();
    let reach = radius_m * std::f64::consts::SQRT_2;
    if let Some(t) = traffic {
        t.roads_near(centre, reach, &mut roads);
    }
    // Tiles that are not resident come from the overview of the whole map.
    overview.roads_near(
        centre,
        reach,
        |key| traffic.is_some_and(|t| t.has_tile(key)),
        &mut roads,
    );
    if let Some(f) = fixture {
        f.roads_near(centre, reach, &mut roads);
    }
    roads
}

/// The route's lines, its stops and the scheduled vehicles in map pixels.
pub(super) type RouteOverlay = (Vec<MapLine>, Vec<(Vec2, bool)>, Vec<(Vec2, f32)>);

/// The route's lines and stops and the scheduled vehicles, in map pixels around the centre.
pub(super) fn route_overlay(
    route: &crate::hud::route_path::PlayerRoutePath,
    scheduled: &crate::hud::route_path::ScheduledVehicles,
    centre: DVec2,
    min_chord: f64,
) -> RouteOverlay {
    let mut route_roads = Vec::new();
    route_roads.extend(route.roads.iter().cloned());
    let mut route_lines: Vec<MapLine> = Vec::new();
    for road in &route_roads {
        push_chords(road, centre, min_chord, &mut route_lines);
    }
    let route_stops: Vec<(Vec2, bool)> = route
        .stops
        .iter()
        .map(|(p, n)| {
            (
                Vec2::new((p.x - centre.x) as f32, (p.y - centre.y) as f32),
                *n,
            )
        })
        .collect();
    let scheduled_px: Vec<(Vec2, f32)> = scheduled
        .poses
        .iter()
        .map(|(p, h)| {
            (
                Vec2::new((p.x - centre.x) as f32, (p.y - centre.y) as f32),
                *h,
            )
        })
        .collect();
    (route_lines, route_stops, scheduled_px)
}

/// The road chords of the map, thinned to what the pixels can show.
pub(super) fn road_lines(
    roads: &[crate::hud::minimap::MapRoad],
    centre: DVec2,
    (min_chord, px_per_m): (f64, f64),
) -> Vec<MapLine> {
    let mut lines: Vec<MapLine> = Vec::with_capacity(roads.len() * 2);
    for road in roads {
        push_chords(road, centre, min_chord, &mut lines);
    }
    thin_lines(&mut lines, px_per_m as f32);
    lines
}

/// The vehicle markers in map pixels around the centre.
pub(super) fn marker_pixels(markers: &[(DVec2, f32, bool)], centre: DVec2) -> Vec<MapMarker> {
    markers
        .iter()
        .map(|(p, h, active)| MapMarker {
            pos: Vec2::new((p.x - centre.x) as f32, (p.y - centre.y) as f32),
            heading: *h,
            active: *active,
        })
        .collect()
}
