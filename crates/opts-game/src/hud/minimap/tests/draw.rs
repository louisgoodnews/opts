//! Drawing tests: the ground, the roads, the headings, the markers, the north mark and the route.

use super::*;

#[test]
fn the_ground_is_opaque_and_untouched_far_from_roads() {
    let mut canvas = Canvas::new(176, [1, 2, 3, 255]);
    render(&mut canvas, NORTH_UP, &[], &[], &colors());
    assert!(canvas.data.chunks_exact(4).all(|p| p[3] == 255));
    assert_eq!(canvas.get(2, 100), colors().ground);
}

#[test]
fn a_road_north_of_the_centre_is_drawn_above_it_and_nowhere_else() {
    let mut canvas = Canvas::new(176, [0; 4]);
    render(
        &mut canvas,
        NORTH_UP,
        &[line((0.0, 0.0), (0.0, 80.0))],
        &[],
        &colors(),
    );
    let c = colors();
    assert_eq!(canvas.get(88, 40), c.road, "road above the centre");
    assert_eq!(canvas.get(88, 130), c.ground, "nothing below the centre");
    assert_eq!(canvas.get(30, 40), c.ground, "nothing beside the road");
}

#[test]
fn heading_up_turns_the_map_with_the_player() {
    // Heading east (90 degrees): a road to the north now runs to the left of the centre.
    let view = MapView {
        radius_m: 100.0,
        up_heading: std::f32::consts::FRAC_PI_2,
    };
    let mut canvas = Canvas::new(176, [0; 4]);
    render(
        &mut canvas,
        view,
        &[line((0.0, 0.0), (0.0, 80.0))],
        &[],
        &colors(),
    );
    let c = colors();
    assert_eq!(canvas.get(40, 88), c.road, "north is to the left");
    assert_eq!(canvas.get(88, 40), c.ground, "nothing straight ahead");
    // A road to the east is straight ahead.
    let mut canvas = Canvas::new(176, [0; 4]);
    render(
        &mut canvas,
        view,
        &[line((0.0, 0.0), (80.0, 0.0))],
        &[],
        &colors(),
    );
    assert_eq!(canvas.get(88, 40), c.road, "east is up");
}

#[test]
fn the_active_vehicle_is_a_filled_arrow_the_others_are_hollow() {
    let c = colors();
    let marker = |active| MapMarker {
        pos: Vec2::ZERO,
        heading: 0.0,
        active,
    };
    let mut a = Canvas::new(176, [0; 4]);
    render(&mut a, NORTH_UP, &[], &[marker(true)], &c);
    let mut o = Canvas::new(176, [0; 4]);
    render(&mut o, NORTH_UP, &[], &[marker(false)], &c);
    assert_eq!(centre_of(&a), c.active, "filled");
    assert_eq!(centre_of(&o), c.ground, "hollow: the inside is the ground");
    // The hollow arrow still has an outline in its own colour.
    assert!(
        o.data.chunks_exact(4).any(|p| p == c.other.as_slice()),
        "outline"
    );
    // The tip points where the vehicle heads: heading north, the tip is above the centre.
    assert_ne!(a.get(88, 82), c.ground);
}

#[test]
fn a_marker_is_visible_on_top_of_a_road() {
    let c = colors();
    let mut canvas = Canvas::new(176, [0; 4]);
    render(
        &mut canvas,
        NORTH_UP,
        &[line((0.0, -90.0), (0.0, 90.0))],
        &[MapMarker {
            pos: Vec2::ZERO,
            heading: 0.0,
            active: true,
        }],
        &c,
    );
    assert_eq!(centre_of(&canvas), c.active);
}

#[test]
fn the_north_mark_follows_north() {
    let c = colors();
    let at_top = |view| {
        let mut canvas = Canvas::new(176, [0; 4]);
        render(&mut canvas, view, &[], &[], &c);
        (0..30).any(|y| (70..106).any(|x| canvas.get(x, y) == c.north))
    };
    assert!(at_top(NORTH_UP), "north up: the mark is at the top");
    assert!(
        !at_top(MapView {
            radius_m: 100.0,
            up_heading: std::f32::consts::PI,
        }),
        "heading south: north is at the bottom"
    );
}

#[test]
fn the_route_is_drawn_over_the_roads_in_its_own_colour_with_its_stops() {
    let colors = colors();
    assert_ne!(colors.route, colors.road);
    let mut canvas = Canvas::new(176, colors.ground);
    let road = [line((-90.0, 0.0), (90.0, 0.0))];
    let route_lines = [line((-90.0, 0.0), (0.0, 0.0))];
    let stops = [(Vec2::new(-40.0, 0.0), true)];
    raster::render_routed(
        &mut canvas,
        NORTH_UP,
        &road,
        MapRoute {
            lines: &route_lines,
            stops: &stops,
            vehicles: &[],
        },
        &[],
        &colors,
        (1.0, false),
    );
    let (x, y) = (canvas.size / 4, canvas.size / 2);
    assert_eq!(canvas.get(x + 30, y)[..3], colors.route[..3]);
    assert_eq!(
        canvas.get(canvas.size * 3 / 4, y)[..3],
        colors.road[..3],
        "the road beyond the route keeps its colour"
    );
}
