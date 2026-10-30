//! The 10 Hz mini-map redraw, caption update, and viewpoint-panel avoidance systems.

use super::*;

/// What the map is centred on and the markers on it, in OMSI metres.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    /// The centre (the active vehicle, else the camera).
    pub centre: Option<DVec2>,
    /// Compass heading (radians) of the centre.
    pub heading: f32,
    /// All markers: (position, heading, active).
    pub markers: Vec<(DVec2, f32, bool)>,
}

/// The player's and the session's vehicles as markers; the player's pose centres the map (the
/// free camera's when there is no vehicle). Also whether there is a player vehicle.
fn gather_scene(
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

/// The view for this tick: the radius (the zoom step, or what the speed asks for: the head unit's
/// road speed, the free camera's own speed without a vehicle) and what points up. Records the
/// radius and speed in the state.
fn choose_view(
    state: &mut MiniMapState,
    (centre, heading): (DVec2, f32),
    has_player: bool,
    hud: Option<&super::super::HudValues>,
    tick_dt: f32,
) -> MapView {
    let config = state.config;
    // The radius: the zoom step, or what the speed asks for (the head unit's road speed; the
    // free camera's own speed when there is no vehicle).
    let moved = state.centre.map(|prev| (centre - prev).length());
    let speed_in = if has_player {
        hud.map(|h| h.speed_kmh)
    } else {
        moved.and_then(|m| zoom::camera_speed_kmh(m, tick_dt))
    };
    if moved.is_some_and(|m| m > zoom::JUMP_M) {
        state.filter.reset();
    }
    let (radius_m, target_m) = match config.zoom_mode {
        ZoomMode::Fixed => {
            state.filter.reset();
            (config.zoom_m, config.zoom_m)
        }
        ZoomMode::Speed => {
            let held = state.filter.speed_kmh;
            let r = state
                .filter
                .step(tick_dt, speed_in.unwrap_or(held), config.zoom_m);
            (
                r,
                zoom::target_radius(state.filter.speed_kmh, config.zoom_m),
            )
        }
    };
    state.radius_m = radius_m;
    state.target_m = target_m;
    state.speed_kmh = state.filter.speed_kmh;
    MapView {
        radius_m,
        up_heading: match config.orientation {
            Orientation::North => 0.0,
            Orientation::Heading => heading,
        },
    }
}

/// The route's lines, its stops and the scheduled vehicles in map pixels.
type RouteOverlay = (Vec<MapLine>, Vec<(Vec2, bool)>, Vec<(Vec2, f32)>);

/// The route's lines and stops and the scheduled vehicles, in map pixels around the centre.
fn route_overlay(
    route: Option<&super::super::route_path::PlayerRoutePath>,
    scheduled: Option<&super::super::route_path::ScheduledVehicles>,
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

/// Redraws the map at most 10 times per second.
#[allow(clippy::too_many_arguments)]
pub fn redraw(
    time: Res<Time<Real>>,
    mut state: ResMut<MiniMapState>,
    mut canvas: ResMut<MiniMapCanvas>,
    handle: Res<MiniMapImage>,
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
    route: Option<Res<super::super::route_path::PlayerRoutePath>>,
    scheduled: Option<Res<super::super::route_path::ScheduledVehicles>>,
    fixture: Option<Res<FixtureRoads>>,
    hud: Option<Res<super::super::HudValues>>,
) {
    state.since += time.delta_secs();
    if !state.config.enabled || state.since < UPDATE_INTERVAL.as_secs_f32() {
        return;
    }
    let (Some(canvas), Some(handle)) = (canvas.0.as_mut(), handle.0.as_ref()) else {
        return;
    };
    let started = std::time::Instant::now();
    let tick_dt = state.since;
    state.since = 0.0;

    let (scene, has_player) = gather_scene(&vehicles, &cameras);
    let Some(centre) = scene.centre else {
        return;
    };

    let colors = colors_of(
        theme
            .as_ref()
            .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                t.instrument(*variant)
            }),
    );
    let view = choose_view(
        &mut state,
        (centre, scene.heading),
        has_player,
        hud.as_deref(),
        tick_dt,
    );
    // Roads: the lane graph's paths, then the fixture (tests).
    let mut roads = Vec::new();
    let radius = f64::from(view.radius_m) * std::f64::consts::SQRT_2;
    if let Some(t) = &traffic {
        t.roads_near(centre, radius, &mut roads);
    }
    if let Some(f) = &fixture {
        f.roads_near(centre, radius, &mut roads);
    }
    let px_per_m = f64::from(canvas.size) / 2.0 / f64::from(view.radius_m);
    let min_chord = 3.0 / px_per_m;
    let mut lines = Vec::with_capacity(roads.len() * 2);
    for road in &roads {
        push_chords(road, centre, min_chord, &mut lines);
    }
    let markers: Vec<MapMarker> = scene
        .markers
        .iter()
        .map(|(p, h, active)| MapMarker {
            pos: Vec2::new((p.x - centre.x) as f32, (p.y - centre.y) as f32),
            heading: *h,
            active: *active,
        })
        .collect();
    let (route_lines, route_stops, scheduled_px) =
        route_overlay(route.as_deref(), scheduled.as_deref(), centre, min_chord);
    raster::render_routed(
        canvas,
        view,
        &lines,
        MapRoute {
            lines: &route_lines,
            stops: &route_stops,
            vehicles: &scheduled_px,
        },
        &markers,
        &colors,
        canvas.size as f32 / 176.0,
        true,
    );
    if let Some(mut image) = images.get_mut(handle) {
        match image.data.as_mut() {
            Some(data) if data.len() == canvas.data.len() => data.copy_from_slice(&canvas.data),
            _ => image.data = Some(canvas.data.clone()),
        }
    }
    let us = started.elapsed().as_micros() as u64;
    state.frames += 1;
    state.last_us = us;
    state.worst_us = state.worst_us.max(us);
    state.lines = lines.len();
    state.markers = markers.len();
    state.centre = Some(centre);
    state.up_heading_deg = view.up_heading.to_degrees().rem_euclid(360.0);
}

/// Writes the caption below the map: the radius and what points up.
pub fn update_caption(
    state: Res<MiniMapState>,
    loc: Res<Localization>,
    mut captions: Query<&mut Text, With<MiniMapCaption>>,
) {
    let mode = match state.config.orientation {
        Orientation::North => keys::MINIMAP_NORTH_UP,
        Orientation::Heading => keys::MINIMAP_HEADING_UP,
    };
    // Speed zoom: the radius being drawn (to 10 m), marked as automatic.
    let (key, metres) = match state.config.zoom_mode {
        ZoomMode::Fixed => (keys::MINIMAP_CAPTION, state.config.zoom_m as i32),
        ZoomMode::Speed => (
            keys::MINIMAP_CAPTION_AUTO,
            if state.radius_m > 0.0 {
                (state.radius_m / 10.0).round() as i32 * 10
            } else {
                state.config.zoom_m as i32
            },
        ),
    };
    let line = loc.t_args(
        key,
        &[
            ("metres".to_owned(), Arg::from(metres)),
            ("mode".to_owned(), Arg::from(loc.t(mode))),
        ],
    );
    for mut text in &mut captions {
        if text.0 != line {
            text.0 = line.clone();
        }
    }
}

/// The viewpoint editor's panel owns the right edge while it is open, so the map steps aside: it
/// is hidden (not removed, so nothing moves) and comes back when the editor closes.
pub fn avoid_viewpoint_panel(
    editor: Option<Res<opts_bevy::ViewpointState>>,
    mut maps: Query<&mut Visibility, With<super::super::parts::MapBox>>,
) {
    let want = if editor.is_some_and(|e| e.editing) {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut vis in &mut maps {
        if *vis != want {
            *vis = want;
        }
    }
}
