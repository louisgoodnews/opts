//! Unit tests of the full-size map: the viewport maths and the system with fixture roads.

use super::viewport::*;
use bevy::math::{DVec2, Vec2};

const SCREEN: Vec2 = Vec2::new(1280.0, 720.0);

fn view() -> Viewport {
    Viewport::new(DVec2::new(1000.0, 2000.0), 500.0)
}

#[test]
fn the_span_is_clamped_to_the_documented_range() {
    assert_eq!(Viewport::new(DVec2::ZERO, 1.0).span_m, MIN_SPAN_M);
    assert_eq!(Viewport::new(DVec2::ZERO, 1e9).span_m, MAX_SPAN_M);
    let mut v = view();
    for _ in 0..100 {
        v.zoom_at(SCREEN / 2.0, SCREEN, 2.0);
    }
    assert_eq!(v.span_m, MIN_SPAN_M);
    for _ in 0..100 {
        v.zoom_at(SCREEN / 2.0, SCREEN, 0.5);
    }
    assert_eq!(v.span_m, MAX_SPAN_M);
}

#[test]
fn the_middle_of_the_screen_is_the_centre_and_north_is_up() {
    let v = view();
    assert!((v.screen_to_world(SCREEN / 2.0, SCREEN) - v.centre).length() < 1e-9);
    let up = v.screen_to_world(Vec2::new(640.0, 0.0), SCREEN);
    assert!(up.y > v.centre.y && (up.x - v.centre.x).abs() < 1e-9);
    // 500 m across the 720 px: the top edge is 250 m north of the centre.
    assert!((up.y - v.centre.y - 250.0).abs() < 1e-6);
}

#[test]
fn world_and_screen_positions_round_trip() {
    let v = view();
    let p = Vec2::new(321.0, 77.0);
    let back = v.world_to_screen(v.screen_to_world(p, SCREEN), SCREEN);
    assert!((back - p).length() < 1e-3);
}

#[test]
fn dragging_pans_by_the_dragged_distance() {
    let mut v = view();
    let grab = Vec2::new(400.0, 300.0);
    let world = v.screen_to_world(grab, SCREEN);
    // Drag 120 px right and 45 px down: the grabbed point must now be under the new cursor.
    let delta = Vec2::new(120.0, 45.0);
    v.pan_pixels(delta, SCREEN);
    let now = v.world_to_screen(world, SCREEN);
    assert!(
        (now - (grab + delta)).length() <= 1.0,
        "off by {:?}",
        now - (grab + delta)
    );
}

#[test]
fn the_wheel_keeps_the_point_under_the_cursor_within_two_pixels() {
    for span in [60.0, 500.0, 5_000.0] {
        for cursor in [Vec2::new(10.0, 10.0), Vec2::new(900.0, 600.0), SCREEN / 2.0] {
            let mut v = Viewport::new(DVec2::new(-3000.0, 800.0), span);
            let under = v.screen_to_world(cursor, SCREEN);
            v.zoom_at(cursor, SCREEN, 1.7);
            let after = v.world_to_screen(under, SCREEN);
            assert!(
                (after - cursor).length() <= 2.0,
                "span {span} cursor {cursor:?}: {after:?}"
            );
        }
    }
}

#[test]
fn zooming_at_the_limit_does_not_move_the_map() {
    let mut v = Viewport::new(DVec2::new(5.0, 6.0), MIN_SPAN_M);
    let before = v;
    v.zoom_at(Vec2::new(100.0, 100.0), SCREEN, 3.0);
    assert_eq!(v, before);
}

#[test]
fn the_scale_scales_with_the_shorter_side_so_a_wide_window_shows_the_same_metres() {
    let v = view();
    let wide = v.px_per_m(Vec2::new(1920.0, 720.0));
    let narrow = v.px_per_m(Vec2::new(720.0, 720.0));
    assert!((wide - narrow).abs() < 1e-12);
    // The canvas covers the longer side: 1280 px at 1.44 px/m is 888.9 m, half of it 444.4 m.
    assert!((v.canvas_radius_m(SCREEN) - 640.0 / (720.0 / 500.0)).abs() < 1e-9);
}

#[test]
fn the_canvas_side_follows_the_screen_within_its_limits() {
    assert_eq!(canvas_px(Vec2::new(100.0, 80.0)), MIN_CANVAS_PX);
    assert_eq!(canvas_px(SCREEN), 1024);
    assert_eq!(canvas_px(Vec2::new(3840.0, 2160.0)), MAX_CANVAS_PX);
}

#[test]
fn a_wheel_notch_is_a_fixed_factor_and_opposite_notches_cancel() {
    assert!((wheel_factor(1.0) - WHEEL_LINE_FACTOR).abs() < 1e-12);
    assert!((wheel_factor(2.0) * wheel_factor(-2.0) - 1.0).abs() < 1e-12);
}

#[test]
fn the_scale_bar_is_a_round_length_below_the_maximum() {
    for (max, want) in [
        (130.0, 100.0),
        (260.0, 200.0),
        (990.0, 500.0),
        (1000.0, 1000.0),
        (7.3, 5.0),
        (0.6, 0.5),
    ] {
        assert_eq!(scale_bar_m(max), want, "max {max}");
    }
    assert_eq!(scale_bar_m(0.0), 1.0);
}

mod overview_tests {
    use super::super::overview::OverviewRoads;
    use crate::hud::minimap::MapRoad;
    use bevy::math::DVec2;
    use opts_traffic::PathGeometry;

    fn road(x: f64, y: f64) -> MapRoad {
        MapRoad {
            geometry: PathGeometry::straight(DVec2::new(x, y), 0.0, 100.0),
            width: 5.0,
        }
    }

    #[test]
    fn roads_come_from_the_tiles_near_the_view_and_not_from_skipped_ones() {
        let mut o = OverviewRoads::default();
        o.insert((0, 0), vec![road(0.0, 0.0)]);
        o.insert((5, 0), vec![road(5_000.0, 0.0)]);
        o.insert((9, 9), vec![road(9_000.0, 9_000.0)]);
        let mut out = Vec::new();
        o.roads_near(DVec2::new(50.0, 50.0), 300.0, |_| false, &mut out);
        assert_eq!(out.len(), 1, "only the tile at the origin is near");
        out.clear();
        o.roads_near(DVec2::new(50.0, 50.0), 300.0, |k| k == (0, 0), &mut out);
        assert!(
            out.is_empty(),
            "a resident tile is drawn from the graph, not twice"
        );
        out.clear();
        o.roads_near(DVec2::ZERO, 20_000.0, |_| false, &mut out);
        assert_eq!(out.len(), 3, "the whole map in view");
    }

    #[test]
    fn progress_counts_tiles_with_and_without_roads() {
        let mut o = OverviewRoads::default();
        assert!(!o.started() && !o.complete());
        o.total = 4;
        assert_eq!(o.percent(), 0);
        o.insert((0, 0), vec![road(0.0, 0.0)]);
        o.insert((1, 0), Vec::new());
        assert_eq!(o.percent(), 50);
        assert_eq!(o.generation, 2, "every finished tile is a change to draw");
    }
}

#[test]
fn fit_shows_the_whole_box_with_a_margin_and_centres_it() {
    let screen = Vec2::new(1920.0, 1000.0);
    let (lo, hi) = (DVec2::new(-3000.0, -1000.0), DVec2::new(3000.0, 1000.0));
    let v = Viewport::fit(lo, hi, screen);
    assert!((v.centre - DVec2::ZERO).length() < 1e-9);
    let a = v.world_to_screen(lo, screen);
    let b = v.world_to_screen(hi, screen);
    for p in [a, b] {
        assert!(
            p.x >= 0.0 && p.x <= screen.x && p.y >= 0.0 && p.y <= screen.y,
            "{p:?} is on screen"
        );
    }
    // The wide side is the limit: 6 km plus a tenth across 1920 px (5 % margin on each side).
    assert!(
        (a.x - 87.3).abs() < 1.0 && (b.x - 1832.7).abs() < 1.0,
        "{a:?} {b:?}"
    );
    // A tiny box is limited by the closest zoom.
    let tiny = Viewport::fit(DVec2::ZERO, DVec2::new(5.0, 5.0), screen);
    assert_eq!(tiny.span_m, MIN_SPAN_M);
}

mod thinning_tests {
    use crate::hud::minimap::MapLine;
    use bevy::math::Vec2;

    fn line(a: (f32, f32), b: (f32, f32)) -> MapLine {
        MapLine {
            a: Vec2::new(a.0, a.1),
            b: Vec2::new(b.0, b.1),
            width_m: 5.0,
        }
    }

    #[test]
    fn sub_pixel_chords_and_covered_chords_are_left_out() {
        // 0.5 px per metre: a 1 m chord is half a pixel, two lanes 0.4 m apart are one line.
        let mut lines = vec![
            line((0.0, 0.0), (100.0, 0.0)),
            line((0.0, 0.2), (100.0, 0.2)),
            line((100.0, 0.0), (0.0, 0.0)),
            line((0.0, 50.0), (1.0, 50.0)),
            line((0.0, 80.0), (100.0, 80.0)),
        ];
        super::super::draw::thin_lines(&mut lines, 0.5);
        assert_eq!(lines.len(), 2, "{lines:?}");
    }
}
