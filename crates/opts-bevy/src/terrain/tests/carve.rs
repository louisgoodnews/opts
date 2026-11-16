//! The carve's rules: an embankment, deep cover, a mirrored profile and a height ramp.

use super::*;

/// `Damm1`-like embankment: crest ±5 m at the datum, flanks down to −20 m at ±35 m, crest 15 m
/// above flat ground. Nothing beyond crest ± 5 m changes and nothing is lowered by more than
/// the guard (today's carve dug a 20 m pit 38 m wide).
#[test]
fn an_embankment_carves_only_its_crest() {
    let ground = flat(10.0);
    let damm = profile(&[
        &[(-5.0, 0.0), (5.0, 0.0)],
        &[(-35.0, -20.0), (-5.0, 0.0)],
        &[(5.0, 0.0), (35.0, -20.0)],
    ]);
    // The crest at 9.5 m: terrain 0.5 m above it is lowered under the crest only.
    let tris = spline_ground_triangles(&damm, &spline(150.0, 50.0, 9.5, 200.0), (0.0, 0.0));
    let (carved, stats) = carve_terrain(&ground, &tris);
    assert!(stats.lowered > 0);
    for j in 0..=60 {
        for i in 0..=60 {
            let (x, d) = (i as f32 * 5.0, h(&ground, i, j) - h(&carved, i, j));
            assert!(d <= CARVE_COVER_M + 1e-5);
            if (x - 150.0).abs() > 10.0 {
                assert_eq!(d, 0.0, "vertex {i},{j} at x {x} lowered by {d}");
            }
        }
    }
    // The crest 15 m above the ground: nothing to carve at all.
    let tris = spline_ground_triangles(&damm, &spline(150.0, 50.0, 25.0, 200.0), (0.0, 0.0));
    assert_eq!(carve_terrain(&ground, &tris).0, ground);
}

/// Three metres of cover over a surface (a tunnel roof): the guard keeps it.
#[test]
fn deep_cover_is_left_alone() {
    let ground = flat(13.0);
    let road = profile(&[&[(-4.0, 0.0), (4.0, 0.0)]]);
    let tris = spline_ground_triangles(&road, &spline(150.0, 50.0, 10.0, 100.0), (0.0, 0.0));
    let (carved, stats) = carve_terrain(&ground, &tris);
    assert_eq!(carved, ground);
    assert!(stats.guarded > 0);
    assert_eq!(stats.lowered, 0);
}

/// An asymmetric profile (surface on the right only) lands on the left when mirrored.
#[test]
fn a_mirrored_profile_carves_the_other_side() {
    let ground = flat(10.2);
    let half = profile(&[&[(2.0, 0.0), (12.0, 0.0)]]);
    let mut inst = spline(150.0, 50.0, 10.0, 100.0);
    let (right, _) = carve_terrain(&ground, &spline_ground_triangles(&half, &inst, (0.0, 0.0)));
    inst.mirrored = true;
    let (left, _) = carve_terrain(&ground, &spline_ground_triangles(&half, &inst, (0.0, 0.0)));
    assert!(right.height(157.0, 100.0) < 10.0);
    assert_eq!(right.height(143.0, 100.0), 10.2);
    assert!(left.height(143.0, 100.0) < 10.0);
    assert_eq!(left.height(157.0, 100.0), 10.2);
}

/// A `[spline_h]` ramp rises 2 m as a cubic; the carve follows the ramp, not a straight line.
#[test]
fn a_spline_h_ramp_is_followed() {
    let ground = flat(11.2);
    let road = profile(&[&[(-4.0, 0.0), (4.0, 0.0)]]);
    let mut inst = spline(150.0, 50.0, 10.0, 100.0);
    inst.has_height_profile = true;
    inst.raw_params = vec![String::new(), String::new(), "2".into()];
    let tris = spline_ground_triangles(&road, &inst, (0.0, 0.0));
    let (carved, _) = carve_terrain(&ground, &tris);
    // Hermite between flat ends: 25 % along, the road is at 10 + 2·(3·0.0625 − 2·0.015625)
    // = 10.3125 (a straight ramp would be at 10.5).
    let at = carved.height(150.0, 75.0);
    assert!(at <= 10.3125 - CARVE_EPSILON_M + 0.05, "{at}");
    assert!(at > 10.0, "{at}");
    // Near the top the terrain (11.2) is within 0.03 + rounding of the road surface.
    assert!(carved.height(150.0, 145.0) > 11.0);
}
