//! Drawing the whole map: roads, the route, the vehicles, the stops and the north mark.

use bevy::math::Vec2;

use super::canvas::Canvas;
use super::lines::{Projection, draw_line};
use super::shapes::{arrow, draw_ring, fill_triangle};
use super::{MapColors, MapLine, MapMarker, MapRoute, MapView};

/// Draws the whole map into `canvas` (which is cleared first). Marker and line sizes scale with
/// the canvas (176 px is one unit, the size of the mini map).
pub fn render(
    canvas: &mut Canvas,
    view: MapView,
    lines: &[MapLine],
    markers: &[MapMarker],
    colors: &MapColors,
) {
    let unit = canvas.size as f32 / 176.0;
    render_with_unit(canvas, view, lines, markers, colors, unit, true);
}

/// [`render`] with an explicit size unit for markers, the north mark and the thinnest line: the
/// full-size map draws a large canvas with markers as big as on the mini map (`unit` about 1.4)
/// and leaves out the north mark (its canvas is larger than the screen, the map is always north-up).
pub fn render_with_unit(
    canvas: &mut Canvas,
    view: MapView,
    lines: &[MapLine],
    markers: &[MapMarker],
    colors: &MapColors,
    unit: f32,
    north_mark: bool,
) {
    render_routed(
        canvas,
        view,
        lines,
        MapRoute::default(),
        markers,
        colors,
        (unit, north_mark),
    );
}

/// [`render_with_unit`] with the player's route drawn over the roads and under the markers.
pub fn render_routed(
    canvas: &mut Canvas,
    view: MapView,
    lines: &[MapLine],
    route: MapRoute,
    markers: &[MapMarker],
    colors: &MapColors,
    (unit, north_mark): (f32, bool),
) {
    canvas.clear(colors.ground);
    let proj = Projection::new(canvas.size, view);
    for l in lines {
        let width = (l.width_m * proj.px_per_m).max(1.6 * unit.max(0.75));
        draw_line(canvas, proj.point(l.a), proj.point(l.b), width, colors.road);
    }
    for l in route.lines {
        let (a, b) = (proj.point(l.a), proj.point(l.b));
        draw_line(canvas, a, b, 3.4 * unit.max(0.75), colors.route);
    }
    for (p, h) in route.vehicles {
        let t = arrow(proj.point(*p), *h - view.up_heading, 6.0 * unit);
        fill_triangle(canvas, t, colors.ground, 2.0 * unit);
        fill_triangle(canvas, t, colors.route, 0.0);
    }
    for (p, next) in route.stops {
        let c = proj.point(*p);
        let r = if *next { 5.0 } else { 3.2 } * unit.max(0.75);
        draw_ring(canvas, c, r, 2.4 * unit.max(0.75), colors.ground);
        draw_ring(canvas, c, r, 1.4 * unit.max(0.75), colors.route);
    }
    // Others first, the active vehicle on top. A ground-coloured halo separates a marker from the
    // roads under it.
    let mut order: Vec<&MapMarker> = markers.iter().collect();
    order.sort_by_key(|m| m.active);
    for m in order {
        let c = proj.point(m.pos);
        let heading = m.heading - view.up_heading;
        if m.active {
            let t = arrow(c, heading, 9.0 * unit);
            fill_triangle(canvas, t, colors.ground, 2.0 * unit);
            draw_ring(canvas, c, 12.5 * unit, 2.0 * unit, colors.ground);
            draw_ring(canvas, c, 12.5 * unit, 1.2 * unit, colors.active);
            fill_triangle(canvas, t, colors.active, 0.0);
        } else {
            // Hollow arrow: the outline is the colour, the inside is the ground.
            let t = arrow(c, heading, 7.0 * unit);
            fill_triangle(canvas, t, colors.ground, 2.2 * unit);
            fill_triangle(canvas, t, colors.other, 1.2 * unit);
            let inner = arrow(c, heading, 7.0 * unit);
            fill_triangle(canvas, inner, colors.ground, -1.4 * unit);
        }
    }
    if !north_mark {
        return;
    }
    // The north mark: a small triangle on the edge at the direction of north.
    let north = -view.up_heading;
    let dir = Vec2::new(north.sin(), -north.cos());
    let at = Vec2::splat(proj.centre) + dir * (proj.centre - 9.0 * unit);
    let t = arrow(at, north, 6.0 * unit);
    fill_triangle(canvas, t, colors.ground, 1.5 * unit);
    fill_triangle(canvas, t, colors.north, 0.0);
}
