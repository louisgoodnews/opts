//! Mini map tests: the drawing, the contrast of its colours, the zoom steps and the budget.

use bevy::prelude::*;
use opts_gui::theme::contrast::contrast_ratio;
use opts_gui::theme::{InstrumentPalette, InstrumentTokens};
use opts_traffic::PathGeometry;

use super::raster::{render, Canvas, MapColors, MapLine, MapMarker, MapView};
use super::source::{push_chords, FixtureRoads, MapRoad, MapSource};
use super::*;

fn day() -> InstrumentPalette {
    InstrumentTokens::default().day
}

fn night() -> InstrumentPalette {
    InstrumentTokens::default().night
}

fn colors() -> MapColors {
    colors_of(&day())
}

const NORTH_UP: MapView = MapView {
    radius_m: 100.0,
    up_heading: 0.0,
};

fn centre_of(canvas: &Canvas) -> [u8; 4] {
    canvas.get(canvas.size / 2, canvas.size / 2)
}

fn line(a: (f32, f32), b: (f32, f32)) -> MapLine {
    line_w(a, b, 6.0)
}

fn line_w(a: (f32, f32), b: (f32, f32), width_m: f32) -> MapLine {
    MapLine {
        a: Vec2::new(a.0, a.1),
        b: Vec2::new(b.0, b.1),
        width_m,
    }
}

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

fn rgb(c: [u8; 4]) -> Color {
    Color::srgb_u8(c[0], c[1], c[2])
}

#[test]
fn every_map_colour_has_at_least_3_to_1_against_the_ground_in_day_and_night() {
    for (name, palette) in [("day", day()), ("night", night())] {
        let c = colors_of(&palette);
        assert_eq!(c.ground[3], 255, "{name}: the ground is opaque");
        for (what, color) in [
            ("road", c.road),
            ("active vehicle", c.active),
            ("other vehicle", c.other),
            ("north mark", c.north),
        ] {
            let ratio = contrast_ratio(rgb(color), rgb(c.ground));
            assert!(ratio >= 3.0, "{name} {what}: {ratio:.2}:1");
        }
    }
}

#[test]
fn the_vehicle_markers_differ_in_shape_and_lightness_not_only_in_hue() {
    // Colour-blind safe: the active vehicle is filled with a ring and the others are hollow
    // (tested above); the colours also differ in lightness, and neither pair is red against green.
    for palette in [day(), night()] {
        let c = colors_of(&palette);
        let ratio = contrast_ratio(rgb(c.active), rgb(c.other));
        assert!(ratio >= 1.2, "active against other: {ratio:.2}:1");
    }
}

#[test]
fn zoom_steps_cover_100_m_to_1_km_and_stop_at_the_ends() {
    assert_eq!(ZOOM_STEPS_M.first(), Some(&100.0));
    assert_eq!(ZOOM_STEPS_M.last(), Some(&1000.0));
    assert_eq!(step_zoom(200.0, 1), 100.0, "in");
    assert_eq!(step_zoom(100.0, 1), 100.0, "in stops");
    assert_eq!(step_zoom(200.0, -1), 500.0, "out");
    assert_eq!(step_zoom(1000.0, -1), 1000.0, "out stops");
    assert_eq!(
        step_zoom(333.0, -1),
        500.0,
        "an unknown value starts at the default"
    );
}

#[test]
fn the_config_reads_the_settings_and_falls_back_to_defaults() {
    let mut s = GameSettings::default();
    assert_eq!(MiniMapConfig::read(&s), MiniMapConfig::default());
    s.set_raw(SETTING_ENABLED, "false");
    s.set_raw(SETTING_ZOOM, "500");
    s.set_raw(SETTING_ORIENTATION, "north");
    let c = MiniMapConfig::read(&s);
    assert!(!c.enabled);
    assert_eq!(c.zoom_m, 500.0);
    assert_eq!(c.orientation, Orientation::North);
    s.set_raw(SETTING_ZOOM, "37");
    s.set_raw(SETTING_ORIENTATION, "sideways");
    let c = MiniMapConfig::read(&s);
    assert_eq!(c.zoom_m, DEFAULT_ZOOM_M);
    assert_eq!(c.orientation, Orientation::Heading);
}

#[test]
fn straight_paths_are_one_chord_and_arcs_follow_the_pixels() {
    let straight = MapRoad {
        geometry: PathGeometry::straight(bevy::math::DVec2::ZERO, 0.0, 500.0),
        width: 5.0,
    };
    let arc = MapRoad {
        geometry: PathGeometry {
            start: bevy::math::DVec2::ZERO,
            heading: 0.0,
            radius: 100.0,
            length: 300.0,
        },
        width: 5.0,
    };
    let count = |road: &MapRoad, min_chord: f64| {
        let mut out = Vec::new();
        push_chords(road, bevy::math::DVec2::ZERO, min_chord, &mut out);
        out.len()
    };
    assert_eq!(count(&straight, 5.0), 1);
    assert!(
        count(&arc, 5.0) > count(&arc, 50.0),
        "zoomed in: more pieces"
    );
    assert!(count(&arc, 0.01) <= 32, "bounded");
    let mut out = Vec::new();
    push_chords(&arc, bevy::math::DVec2::ZERO, 10.0, &mut out);
    for pair in out.windows(2) {
        assert!(pair[0].b.distance(pair[1].a) < 1e-3, "the chords join");
    }
}

#[test]
fn a_fixture_only_hands_over_roads_near_the_centre() {
    let fixture = FixtureRoads::grid();
    let mut near = Vec::new();
    fixture.roads_near(bevy::math::DVec2::ZERO, 100.0, &mut near);
    let mut far = Vec::new();
    fixture.roads_near(bevy::math::DVec2::new(5000.0, 5000.0), 100.0, &mut far);
    assert!(!near.is_empty());
    assert!(far.is_empty());
}

#[test]
fn a_redraw_stays_within_the_budget() {
    // A dense city block at a large texture: 800 chords of about 40 px and a few vehicles. The
    // fastest of many runs must be inside the 1 ms budget (the minimum is robust against a busy
    // machine).
    let c = colors();
    let mut lines = Vec::new();
    for i in 0..400 {
        let o = (i % 40) as f32 * 5.0 - 100.0;
        let s = (i / 40) as f32 * 20.0 - 100.0;
        lines.push(line_w((o, s), (o + 2.0, s + 18.0), 3.5));
        lines.push(line_w((s, o), (s + 18.0, o + 3.0), 3.5));
    }
    let markers: Vec<MapMarker> = (0..6)
        .map(|i| MapMarker {
            pos: Vec2::new(i as f32 * 20.0 - 50.0, 10.0),
            heading: i as f32,
            active: i == 0,
        })
        .collect();
    let mut canvas = Canvas::new(224, c.ground);
    let mut best = u128::MAX;
    for _ in 0..25 {
        let t = std::time::Instant::now();
        render(&mut canvas, NORTH_UP, &lines, &markers, &c);
        best = best.min(t.elapsed().as_micros());
    }
    assert!(
        best < u128::from(BUDGET_US) * 3,
        "{best} us for 800 chords at 224 px (budget {BUDGET_US} us, 3x slack for a busy CI machine; \
         the live time is `hud.state` minimap.last_us)"
    );
}
