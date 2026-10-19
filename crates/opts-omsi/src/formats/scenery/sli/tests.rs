//! Unit tests of `sli.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

#[test]
fn flat_width_counts_only_level_strips() {
    let pt = |x: f32, z: f32| SplineProfilePoint {
        x,
        z,
        u: 0.0,
        v_slope: 0.0,
    };
    let mut sli = SplineProfile::default();
    // A paved area: 10 m flat; a kerb: 0.15 m wide with 0.15 m rise; a wall: vertical.
    sli.graphical_profiles.push(SplineGraphicalProfile {
        texture_index: 0,
        points: vec![pt(-5.0, 0.0), pt(5.0, 0.0), pt(5.0, 0.4)],
    });
    assert!((sli.flat_width() - 10.0).abs() < 1e-4);
    sli.graphical_profiles[0].points = vec![pt(0.0, 0.0), pt(0.0, 2.0), pt(0.2, 2.0)];
    assert!(sli.flat_width() < 0.5);
}

use super::*;

fn pts(points: &[(f32, f32)]) -> SplineGraphicalProfile {
    SplineGraphicalProfile {
        texture_index: 0,
        points: points
            .iter()
            .map(|&(x, z)| SplineProfilePoint {
                x,
                z,
                u: 0.0,
                v_slope: 0.0,
            })
            .collect(),
    }
}

/// `Damm1.sli`: a 10 m crest at the datum and grass flanks down to −20 m at ±35 m. Only the
/// crest is ground surface (the flanks slope 0.67).
#[test]
fn an_embankment_surface_is_its_crest() {
    let mut sli = SplineProfile::default();
    sli.graphical_profiles.push(pts(&[(-5.0, 0.0), (5.0, 0.0)]));
    sli.graphical_profiles
        .push(pts(&[(-35.0, -20.0), (-5.0, 0.0)]));
    sli.graphical_profiles
        .push(pts(&[(5.0, 0.0), (35.0, -20.0)]));
    let s = sli.ground_surface();
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!((s[0].x0, s[0].x1), (-5.0, 5.0));
    assert_eq!(sli.surface_at(0.0), Some(0.0));
    assert_eq!(sli.surface_at(20.0), None);
}

/// Street with kerbs: carriageway 0.0, kerb faces (vertical), pavements +0.15; a rail head
/// over the ballast and a wall top 2 m up drop out; a segment drawn right to left (an
/// underside) does not count.
#[test]
fn the_surface_is_the_lowest_up_facing_level_segment() {
    let mut sli = SplineProfile::default();
    sli.graphical_profiles.push(pts(&[
        (-6.0, 0.15),
        (-4.0, 0.15),
        (-4.0, 0.0),
        (4.0, 0.0),
        (4.0, 0.15),
        (6.0, 0.15),
    ]));
    sli.graphical_profiles.push(pts(&[(-1.0, 0.2), (1.0, 0.2)]));
    sli.graphical_profiles.push(pts(&[(6.0, 2.0), (6.5, 2.0)]));
    sli.graphical_profiles
        .push(pts(&[(3.0, -1.0), (-3.0, -1.0)]));
    let s = sli.ground_surface();
    assert_eq!(s.len(), 3, "{s:?}");
    assert_eq!(sli.surface_at(0.0), Some(0.0));
    assert_eq!(sli.surface_at(5.0), Some(0.15));
    assert_eq!(sli.surface_at(6.25), None);
    // Crossing segments: the envelope switches at the intersection.
    let mut x = SplineProfile::default();
    x.graphical_profiles.push(pts(&[(-2.0, 0.0), (2.0, 0.4)]));
    x.graphical_profiles.push(pts(&[(-2.0, 0.4), (2.0, 0.0)]));
    let s = x.ground_surface();
    assert_eq!(s.len(), 2, "{s:?}");
    assert!((x.surface_at(0.0).unwrap() - 0.2).abs() < 1e-5);
    assert!((x.surface_at(-2.0).unwrap()).abs() < 1e-5);
    assert!((x.surface_at(2.0).unwrap()).abs() < 1e-5);
    // Editor-only profiles lay nothing down.
    x.only_editor = true;
    assert!(x.ground_surface().is_empty());
}

#[test]
fn test_parse_minimal_sli() {
    let text = r#"
[length]
10.0

[heightprofile]
-5.750
-3.750
0.250
0.250

[heightprofile]
-3.750
3.750
0.100
0.100

[texture]
str_gehweg02.bmp

[texture]
str_asphdrk_1line.bmp

[profile]
1

[profilepnt]
-3.750
0.100
0.995
0.167

[profilepnt]
0.000
0.100
0.005
0.167

[path]
0
-2.5
0.125
5.0
1

[path]
0
2.5
0.125
5.0
0
"#;
    let sli = SplineProfile::from_text(text).unwrap();
    assert_eq!(sli.length, Some(10.0));
    assert_eq!(sli.height_profiles.len(), 2);
    assert_eq!(sli.height_profiles[0].x_start, -5.75);
    assert_eq!(sli.height_profiles[0].z_start, 0.25);

    assert_eq!(sli.textures.len(), 2);
    assert_eq!(sli.textures[0].file, "str_gehweg02.bmp");
    assert_eq!(sli.textures[1].file, "str_asphdrk_1line.bmp");

    assert_eq!(sli.graphical_profiles.len(), 1);
    assert_eq!(sli.graphical_profiles[0].texture_index, 1);
    assert_eq!(sli.graphical_profiles[0].points.len(), 2);
    assert_eq!(sli.graphical_profiles[0].points[0].x, -3.75);
    assert_eq!(sli.graphical_profiles[0].points[0].z, 0.1);
    assert_eq!(sli.graphical_profiles[0].points[0].u, 0.995);
    assert_eq!(sli.graphical_profiles[0].points[0].v_slope, 0.167);

    assert_eq!(sli.paths.len(), 2);
    assert_eq!(sli.paths[0].path_type, 0);
    assert_eq!(sli.paths[0].offset_x, -2.5);
    assert_eq!(sli.paths[0].direction, 1);
    assert_eq!(sli.paths[1].offset_x, 2.5);
    assert_eq!(sli.paths[1].direction, 0);
}
