//! Drawing the full map into its canvas and uploading the image.

use super::super::*;
use super::inputs::{collect_roads, marker_pixels, road_lines, route_overlay};

/// A fresh canvas and the image that shows it.
pub(super) fn new_canvas(side: u32, ground: [u8; 4]) -> (Canvas, Image) {
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

/// The map colours of the current theme and variant.
pub(super) fn look_colors(look: &super::super::params::MapLook) -> crate::hud::minimap::MapColors {
    colors_of(
        look.theme
            .as_ref()
            .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                t.instrument(*look.variant)
            }),
    )
}

/// Copies the canvas into the image the surface shows.
pub(super) fn upload_image(images: &mut Assets<Image>, handle: &Handle<Image>, canvas: &Canvas) {
    if let Some(mut image) = images.get_mut(handle) {
        match image.data.as_mut() {
            Some(data) if data.len() == canvas.data.len() => data.copy_from_slice(&canvas.data),
            _ => image.data = Some(canvas.data.clone()),
        }
    }
}

/// Draws roads, route and markers onto the canvas; returns how many lines and markers it drew.
pub(super) fn draw_scene(
    canvas: &mut Canvas,
    (viewport, screen): (Viewport, Vec2),
    traffic_fixture: (Option<&TrafficState>, Option<&FixtureRoads>),
    (overview, route, scheduled): (
        &OverviewRoads,
        &crate::hud::route_path::PlayerRoutePath,
        &crate::hud::route_path::ScheduledVehicles,
    ),
    (markers, colors): (&[(DVec2, f32, bool)], &crate::hud::minimap::MapColors),
) -> (usize, usize) {
    let radius_m = viewport.canvas_radius_m(screen);
    let view = MapView {
        radius_m: radius_m as f32,
        up_heading: 0.0,
    };
    let roads = collect_roads(viewport.centre, radius_m, traffic_fixture, overview);
    let px_per_m = f64::from(canvas.size) / 2.0 / radius_m;
    // Chords of at least six pixels: the large canvas has many arcs, and a six-pixel chord is
    // still within a fifth of a pixel of the arc (this keeps a redraw near the 2 ms budget).
    let min_chord = MIN_CHORD_PX / px_per_m;
    let lines = road_lines(&roads, viewport.centre, (min_chord, px_per_m));
    let drawn_markers = marker_pixels(markers, viewport.centre);
    let (route_lines, route_stops, scheduled_px) =
        route_overlay(route, scheduled, viewport.centre, min_chord);
    raster::render_routed(
        canvas,
        view,
        &lines,
        crate::hud::minimap::MapRoute {
            lines: &route_lines,
            stops: &route_stops,
            vehicles: &scheduled_px,
        },
        &drawn_markers,
        colors,
        (MARKER_UNIT, false),
    );
    (lines.len(), drawn_markers.len())
}
