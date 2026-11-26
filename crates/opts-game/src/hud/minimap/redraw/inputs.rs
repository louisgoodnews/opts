//! The inputs of the mini map's redraw: the markers, the roads and the route, in map pixels.

use super::super::*;

/// The player's and the session's vehicles as markers; the player's pose centres the map (the
/// free camera's when there is no vehicle). Also whether there is a player vehicle.
pub(super) fn gather_scene(
    vehicles: &Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: &Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) -> (Scene, bool) {
    let mut scene = Scene::default();
    let mut has_player = false;
    for (t, session, is_player) in vehicles {
        if session.is_none() && !is_player {
            continue;
        }
        let (pos, heading) = pose_of(t);
        if is_player {
            scene.centre = Some(pos);
            scene.heading = heading;
            has_player = true;
        }
        scene.markers.push((pos, heading, is_player));
    }
    if scene.centre.is_none() {
        if let Some(t) = cameras.iter().next() {
            let (pos, heading) = pose_of(t);
            scene.centre = Some(pos);
            scene.heading = heading;
            scene.markers.push((pos, heading, true));
        }
    }
    (scene, has_player)
}

/// The timetable course's chords around the centre, in map pixels (none without a course).
pub(super) fn course_overlay(
    route: Option<&crate::hud::route_path::PlayerRoutePath>,
    centre: DVec2,
    min_chord: f64,
) -> Vec<MapLine> {
    let mut lines = Vec::new();
    if let Some(r) = route {
        for road in &r.course {
            push_chords(road, centre, min_chord, &mut lines);
        }
    }
    lines
}

/// The route's lines, its stops and the scheduled vehicles in map pixels.
pub(super) type RouteOverlay = (Vec<MapLine>, Vec<(Vec2, bool)>, Vec<(Vec2, f32)>);

/// The route's lines and stops and the scheduled vehicles, in map pixels around the centre.
pub(super) fn route_overlay(
    route: Option<&crate::hud::route_path::PlayerRoutePath>,
    scheduled: Option<&crate::hud::route_path::ScheduledVehicles>,
    centre: DVec2,
    min_chord: f64,
) -> RouteOverlay {
    let mut route_lines: Vec<MapLine> = Vec::new();
    let mut route_stops: Vec<(Vec2, bool)> = Vec::new();
    if let Some(r) = route {
        for road in &r.roads {
            push_chords(road, centre, min_chord, &mut route_lines);
        }
        route_stops.extend(r.stops.iter().map(|(p, n)| {
            (
                Vec2::new((p.x - centre.x) as f32, (p.y - centre.y) as f32),
                *n,
            )
        }));
    }
    let scheduled_px: Vec<(Vec2, f32)> = scheduled.map_or_else(Vec::new, |s| {
        s.poses
            .iter()
            .map(|(p, h)| {
                (
                    Vec2::new((p.x - centre.x) as f32, (p.y - centre.y) as f32),
                    *h,
                )
            })
            .collect()
    });
    (route_lines, route_stops, scheduled_px)
}

/// The road chords around the centre: the lane graph's paths, then the fixture (tests).
pub(super) fn road_lines(
    (traffic, fixture): (Option<&TrafficState>, Option<&FixtureRoads>),
    centre: DVec2,
    (radius_m, min_chord): (f64, f64),
) -> Vec<MapLine> {
    let mut roads = Vec::new();
    let radius = radius_m * std::f64::consts::SQRT_2;
    if let Some(t) = traffic {
        t.roads_near(centre, radius, &mut roads);
    }
    if let Some(f) = fixture {
        f.roads_near(centre, radius, &mut roads);
    }
    let mut lines = Vec::with_capacity(roads.len() * 2);
    for road in &roads {
        push_chords(road, centre, min_chord, &mut lines);
    }
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
