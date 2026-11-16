//! Unit tests for framing.

use super::*;

fn unit_box() -> Bounds {
    Bounds {
        min: Vec3::new(-1.0, -1.0, 0.0),
        max: Vec3::new(1.0, 1.0, 2.0),
    }
}

#[test]
fn top_view_puts_north_up_and_east_right() {
    let v = top_view(&unit_box());
    let centre = v.project(map_to_bevy(Vec3::new(0.0, 0.0, 1.0))).unwrap();
    let east = v.project(map_to_bevy(Vec3::new(1.0, 0.0, 1.0))).unwrap();
    let north = v.project(map_to_bevy(Vec3::new(0.0, 1.0, 1.0))).unwrap();
    assert!((centre - Vec2::splat(256.0)).length() < 1e-3);
    assert!(east.x > centre.x && (east.y - centre.y).abs() < 1e-3);
    assert!(north.y < centre.y && (north.x - centre.x).abs() < 1e-3);
}

#[test]
fn text_views_look_at_both_sides_of_a_wall() {
    let wall = [[
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(4.0, 0.0, 2.0),
    ]];
    let [a, b] = text_views(&wall).unwrap();
    assert!((a.eye - b.eye).length() > 1.0);
    let c = map_to_bevy(Vec3::new(2.0, 0.0, 1.0));
    assert!(a.project(c).unwrap().distance(Vec2::splat(256.0)) < 40.0);
    assert!(text_views(&[]).is_none());
    // A back-to-back duplicate must not cancel the normal.
    let mut both = wall.to_vec();
    both.push([wall[0][0], wall[0][2], wall[0][1]]);
    let [c, _] = text_views(&both).unwrap();
    assert!((c.eye - a.eye).length() < 1e-3 || (c.eye - b.eye).length() < 1e-3);
}

#[test]
fn oblique_view_keeps_the_sphere_inside_the_image() {
    let b = unit_box();
    let v = oblique_view(&b, 37.0);
    let c = v.project(map_to_bevy(b.centre())).unwrap();
    assert!((c - Vec2::splat(256.0)).length() < 1e-2);
    for corner in [b.min, b.max, Vec3::new(b.min.x, b.max.y, b.max.z)] {
        let p = v.project(map_to_bevy(corner)).unwrap();
        assert!(p.x > 0.0 && p.x < 512.0 && p.y > 0.0 && p.y < 512.0);
    }
}
