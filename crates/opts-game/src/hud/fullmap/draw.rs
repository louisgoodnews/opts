//! The texture of the full-size map and its redraw.

use super::overview::OverviewRoads;
use super::viewport::canvas_px;
use super::*;

/// The texture's drawing buffer (kept between redraws).
#[derive(Resource, Default)]
pub struct FullMapCanvas(pub(super) Option<Canvas>);

/// The image the surface shows.
#[derive(Resource, Default)]
pub struct FullMapImage(pub(super) Option<Handle<Image>>);

/// The player's and the session's vehicles as markers, or the free camera when there is no player.
fn gather_markers(
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
fn collect_roads(
    centre: DVec2,
    radius_m: f64,
    (traffic, fixture): (Option<&TrafficState>, Option<&FixtureRoads>),
    overview: &OverviewRoads,
) -> Vec<super::super::minimap::MapRoad> {
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
type RouteOverlay = (Vec<MapLine>, Vec<(Vec2, bool)>, Vec<(Vec2, f32)>);

/// The route's lines and stops and the scheduled vehicles, in map pixels around the centre.
fn route_overlay(
    route: &super::super::route_path::PlayerRoutePath,
    scheduled: &super::super::route_path::ScheduledVehicles,
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

/// Whether the map is drawn now: at once after a resize or a change of the view or the markers
/// while something moves (at most ten times a second), otherwise twice a second.
fn redraw_due(
    state: &FullMapState,
    generation: u64,
    resized: bool,
    markers: &[(DVec2, f32, bool)],
) -> bool {
    let view_changed = state.drawn_generation != generation
        || state
            .drawn
            .as_ref()
            .is_none_or(|(v, _)| *v != state.viewport);
    let markers_moved = state.drawn.as_ref().is_none_or(|(_, old)| {
        old.len() != markers.len()
            || old
                .iter()
                .zip(markers)
                .any(|(a, b)| a.0.distance(b.0) > 0.05 || (a.1 - b.1).abs() > 0.01 || a.2 != b.2)
    });
    let due = resized
        || state.immediate
        || (view_changed && state.since >= MOVING_INTERVAL_S)
        || (markers_moved && state.since >= MOVING_INTERVAL_S)
        || state.since >= IDLE_INTERVAL_S;
    due
}

/// A fresh canvas and the image that shows it.
fn new_canvas(side: u32, ground: [u8; 4]) -> (Canvas, Image) {
    let c = Canvas::new(side, ground);
    let image = Image::new(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        c.data.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    (c, image)
}

/// The road chords of the map, thinned to what the pixels can show.
fn road_lines(
    roads: &[super::super::minimap::MapRoad],
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
fn marker_pixels(markers: &[(DVec2, f32, bool)], centre: DVec2) -> Vec<MapMarker> {
    markers
        .iter()
        .map(|(p, h, active)| MapMarker {
            pos: Vec2::new(
                (p.x - centre.x) as f32,
                (p.y - centre.y) as f32,
            ),
            heading: *h,
            active: *active,
        })
        .collect()
}

/// What the state remembers of this redraw.
fn record_redraw(
    state: &mut FullMapState,
    started: std::time::Instant,
    (lines, drawn): (usize, usize),
    (viewport, markers, generation): (Viewport, Vec<(DVec2, f32, bool)>, u64),
) {
    let us = started.elapsed().as_micros() as u64;
    state.frames += 1;
    state.last_us = us;
    state.worst_us = state.worst_us.max(us);
    state.lines = lines;
    state.markers = drawn;
    state.drawn = Some((viewport, markers));
    state.drawn_generation = generation;
}

/// Redraws the map: at once after a change of the view, while a vehicle moves at most ten times
/// a second, otherwise twice a second.
#[allow(clippy::too_many_arguments)]
pub fn redraw(
    time: Res<Time<Real>>,
    mut state: ResMut<FullMapState>,
    mut canvas: ResMut<FullMapCanvas>,
    mut handle: ResMut<FullMapImage>,
    mut images: ResMut<Assets<Image>>,
    variant: Res<InstrumentVariant>,
    theme: Option<Res<MaterialTheme>>,
    vehicles: Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    traffic: Option<Res<TrafficState>>,
    fixture: Option<Res<FixtureRoads>>,
    mut overview: ResMut<OverviewRoads>,
    route: Res<super::super::route_path::PlayerRoutePath>,
    scheduled: Res<super::super::route_path::ScheduledVehicles>,
) {
    overview.poll();
    state.since += time.delta_secs();
    let markers = gather_markers(&vehicles, &cameras);
    let screen = state.screen;
    let side = canvas_px(screen);
    let resized = canvas.0.as_ref().is_none_or(|c| c.size != side) || handle.0.is_none();
    let due = redraw_due(&state, overview.generation + route.generation, resized, &markers);
    if !due {
        return;
    }
    let started = std::time::Instant::now();
    let colors = colors_of(
        theme
            .as_ref()
            .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                t.instrument(*variant)
            }),
    );
    if resized {
        let (c, image) = new_canvas(side, colors.ground);
        handle.0 = Some(images.add(image));
        canvas.0 = Some(c);
        state.texture_px = side;
    }
    let (Some(canvas), Some(handle)) = (canvas.0.as_mut(), handle.0.as_ref()) else {
        return;
    };
    state.since = 0.0;
    state.immediate = false;

    let viewport = state.viewport;
    let radius_m = viewport.canvas_radius_m(screen);
    let view = MapView {
        radius_m: radius_m as f32,
        up_heading: 0.0,
    };
    let roads = collect_roads(
        viewport.centre,
        radius_m,
        (traffic.as_deref(), fixture.as_deref()),
        &overview,
    );
    let px_per_m = f64::from(canvas.size) / 2.0 / radius_m;
    // Chords of at least six pixels: the large canvas has many arcs, and a six-pixel chord is
    // still within a fifth of a pixel of the arc (this keeps a redraw near the 2 ms budget).
    let min_chord = MIN_CHORD_PX / px_per_m;
    let lines = road_lines(&roads, viewport.centre, (min_chord, px_per_m));
    let drawn_markers = marker_pixels(&markers, viewport.centre);
    let (route_lines, route_stops, scheduled_px) =
        route_overlay(&route, &scheduled, viewport.centre, min_chord);
    raster::render_routed(
        canvas,
        view,
        &lines,
        super::super::minimap::MapRoute {
            lines: &route_lines,
            stops: &route_stops,
            vehicles: &scheduled_px,
        },
        &drawn_markers,
        &colors,
        MARKER_UNIT,
        false,
    );
    if let Some(mut image) = images.get_mut(handle) {
        match image.data.as_mut() {
            Some(data) if data.len() == canvas.data.len() => data.copy_from_slice(&canvas.data),
            _ => image.data = Some(canvas.data.clone()),
        }
    }
    record_redraw(
        &mut state,
        started,
        (lines.len(), drawn_markers.len()),
        (viewport, markers, overview.generation + route.generation),
    );
}

/// Leaves out what cannot be seen: chords shorter than a pixel, and a chord that another one
/// already covers at the pixel level (the two lanes of a street, or two paths over each other, once
/// the map is zoomed out). The whole map has thousands of chords; this keeps a redraw cheap.
pub(in super::super::super) fn thin_lines(lines: &mut Vec<MapLine>, px_per_m: f32) {
    let mut seen = std::collections::HashSet::with_capacity(lines.len());
    lines.retain(|l| {
        let len_px = (l.b - l.a).length() * px_per_m;
        if len_px < 1.0 {
            return false;
        }
        // Half-pixel cells; the two ends in a fixed order, so both directions of a lane match.
        let q = |p: Vec2| {
            (
                (p.x * px_per_m * 2.0).round() as i32,
                (p.y * px_per_m * 2.0).round() as i32,
            )
        };
        let (a, b) = (q(l.a), q(l.b));
        seen.insert(if a <= b { (a, b) } else { (b, a) })
    });
}
