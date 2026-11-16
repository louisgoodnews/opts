//! Tests of the line thinning: sub-pixel chords and chords covered by another one go.

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
