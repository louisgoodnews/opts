//! Pane geometry and the UV transform of the picture.

use super::*;

#[test]
fn the_geometry_of_a_pane_is_its_centre_normal_height_and_aspect() {
    let pos = [
        [0.0, 0.0, 0.0],
        [0.4, 0.0, 0.0],
        [0.4, 0.2, 0.0],
        [0.0, 0.2, 0.0],
    ];
    let nor = [[0.0, 0.0, 1.0]; 4];
    let (c, n, h, aspect) = pane_geometry(&pos, &nor).unwrap();
    assert!(close(c, Vec3::new(0.2, 0.1, 0.0)) && close(n, Vec3::Z));
    assert!((h - 0.2).abs() < 1e-6, "extent along the up direction");
    assert!((aspect - 2.0).abs() < 1e-5, "{aspect}");
    // A portrait pane tilted back about X keeps its in-plane height.
    let tilt = Quat::from_rotation_x(0.3);
    let pos2: Vec<[f32; 3]> = [
        [0.0, 0.0, 0.0],
        [0.2, 0.0, 0.0],
        [0.2, 0.4, 0.0],
        [0.0, 0.4, 0.0],
    ]
    .iter()
    .map(|p| (tilt * Vec3::from(*p)).to_array())
    .collect();
    let nor2 = vec![(tilt * Vec3::Z).to_array(); 4];
    let (_, _, h2, a2) = pane_geometry(&pos2, &nor2).unwrap();
    assert!(
        (h2 - 0.4).abs() < 1e-4 && (a2 - 0.5).abs() < 1e-4,
        "{h2} {a2}"
    );
    assert!(pane_geometry(&[], &[]).is_none());
    assert!(pane_geometry(&pos, &[]).is_none());
    // A pane facing up uses the X axis as its up direction.
    let flat: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.5],
        [0.0, 0.0, 0.5],
    ];
    let (_, _, hf, af) = pane_geometry(&flat, &[[0.0, 1.0, 0.0]; 4]).unwrap();
    assert!(
        (hf - 1.0).abs() < 1e-5 && (af - 0.5).abs() < 1e-5,
        "{hf} {af}"
    );
}

#[test]
fn the_uv_rectangle_of_the_pane_is_stretched_over_the_picture() {
    let (lo, hi) = uv_bounds(&[[0.16, -0.7], [0.89, -0.26], [0.5, -0.4]]).unwrap();
    assert!(close2(lo, Vec2::new(0.16, -0.7)) && close2(hi, Vec2::new(0.89, -0.26)));
    assert!(uv_bounds(&[]).is_none());
    let t = pane_uv_transform(lo, hi, false, false);
    assert!(close2(t.transform_point2(lo), Vec2::ZERO));
    assert!(close2(t.transform_point2(hi), Vec2::ONE));
    let f = pane_uv_transform(lo, hi, true, false);
    assert!(
        close2(f.transform_point2(lo), Vec2::new(1.0, 0.0)),
        "mirrored in u"
    );
    assert!(close2(f.transform_point2(hi), Vec2::new(0.0, 1.0)));
    let g = pane_uv_transform(lo, hi, false, true);
    assert!(
        close2(g.transform_point2(lo), Vec2::new(0.0, 1.0)),
        "turned in v"
    );
    // A degenerate rectangle does not blow up.
    let d = pane_uv_transform(Vec2::splat(0.5), Vec2::splat(0.5), false, false);
    assert!(d.transform_point2(Vec2::splat(0.5)).is_finite());
}
