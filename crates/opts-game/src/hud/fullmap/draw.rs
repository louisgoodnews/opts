//! The texture of the full-size map and its redraw.

use super::overview::OverviewRoads;
use super::viewport::canvas_px;
use super::*;

mod inputs;
mod pacing;
mod render;

use inputs::gather_markers;
use pacing::{record_redraw, redraw_due};
use render::{draw_scene, look_colors, new_canvas, upload_image};

/// The texture's drawing buffer (kept between redraws).
#[derive(Resource, Default)]
pub struct FullMapCanvas(pub(super) Option<Canvas>);

/// The image the surface shows.
#[derive(Resource, Default)]
pub struct FullMapImage(pub(super) Option<Handle<Image>>);

/// Redraws the map: at once after a change of the view, while a vehicle moves at most ten times
/// a second, otherwise twice a second.
pub fn redraw(
    time: Res<Time<Real>>,
    mut state: ResMut<FullMapState>,
    texture: super::params::MapTexture,
    look: super::params::MapLook,
    poses: super::params::MapPoses,
    sources: super::params::MapSources,
    mut overview: ResMut<OverviewRoads>,
) {
    let super::params::MapTexture {
        mut canvas,
        mut handle,
        mut images,
    } = texture;
    let super::params::MapPoses { vehicles, cameras } = poses;
    let super::params::MapSources {
        traffic,
        fixture,
        route,
        scheduled,
    } = sources;
    overview.poll();
    state.since += time.delta_secs();
    let markers = gather_markers(&vehicles, &cameras);
    let screen = state.screen;
    let side = canvas_px(screen);
    let resized = canvas.0.as_ref().is_none_or(|c| c.size != side) || handle.0.is_none();
    let due = redraw_due(
        &state,
        overview.generation + route.generation,
        resized,
        &markers,
    );
    if !due {
        return;
    }
    let started = std::time::Instant::now();
    let colors = look_colors(&look);
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
    let (lines, drawn_markers) = draw_scene(
        canvas,
        (viewport, state.screen),
        (traffic.as_deref(), fixture.as_deref()),
        (&overview, &route, &scheduled),
        (&markers, &colors),
    );
    upload_image(&mut images, handle, canvas);
    record_redraw(
        &mut state,
        started,
        (lines, drawn_markers),
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
