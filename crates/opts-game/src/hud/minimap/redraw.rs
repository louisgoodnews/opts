//! The 10 Hz mini-map redraw, caption update, and viewpoint-panel avoidance systems.

use super::*;
mod avoid;
mod caption;
mod inputs;
mod render;
mod view;

use inputs::{course_overlay, gather_scene, marker_pixels, road_lines, route_overlay};
use render::{look_colors, look_course_color, record, upload};
use view::choose_view;

pub use avoid::avoid_viewpoint_panel;
pub use caption::update_caption;

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

/// Redraws the map at most 10 times per second.
pub fn redraw(
    time: Res<Time<Real>>,
    mut state: ResMut<MiniMapState>,
    texture: super::params::MiniTexture,
    look: super::params::MapLook,
    poses: super::params::MapPoses,
    sources: super::params::MiniSources,
) {
    let super::params::MiniTexture {
        mut canvas,
        handle,
        mut images,
    } = texture;
    let super::params::MapPoses { vehicles, cameras } = poses;
    let super::params::MiniSources {
        traffic,
        route,
        scheduled,
        fixture,
        hud,
    } = sources;
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

    let colors = look_colors(&look);

    let view = choose_view(
        &mut state,
        (centre, scene.heading),
        has_player,
        hud.as_deref(),
        tick_dt,
    );
    let px_per_m = f64::from(canvas.size) / 2.0 / f64::from(view.radius_m);
    let min_chord = 3.0 / px_per_m;
    let lines = road_lines(
        (traffic.as_deref(), fixture.as_deref()),
        centre,
        (f64::from(view.radius_m), min_chord),
    );
    let markers = marker_pixels(&scene.markers, centre);
    let (route_lines, route_stops, scheduled_px) =
        route_overlay(route.as_deref(), scheduled.as_deref(), centre, min_chord);
    let course_px = course_overlay(route.as_deref(), centre, min_chord);
    raster::render_course(
        canvas,
        view,
        &lines,
        (
            MapRoute {
                lines: &route_lines,
                stops: &route_stops,
                vehicles: &scheduled_px,
            },
            MapCourse {
                lines: &course_px,
                color: look_course_color(&look),
            },
        ),
        &markers,
        &colors,
        (canvas.size as f32 / 176.0, true),
    );
    upload(&mut images, handle, canvas);
    record(
        &mut state,
        started,
        (lines.len(), markers.len()),
        (centre, view.up_heading),
    );
}
