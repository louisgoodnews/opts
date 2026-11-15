//! `.sli` spline extrusion profiles of the world parse tests (`opts_omsi::formats::scenery::sli`).

use opts_omsi::formats::scenery::SplineProfile;

#[test]
fn test_spline_profile_full_feature_parsing() {
    let sli_text = r#"
[length]
7.5

[halfcantwidth]
1.435

[scaleTexByLength]

[rendertype]
presurface

[heightprofile]
-6.0
-4.0
0.2
0.2

[heightprofile]
-4.0
4.0
0.0
0.0

[heightprofile]
4.0
6.0
0.2
0.2

[texture]
curb.bmp

[texture]
asphalt.bmp

[profile]
0

[profilepnt]
-6.0
0.2
0.0
0.2

[profilepnt]
-4.0
0.2
1.0
0.2

[profile]
1

[profilepnt]
-4.0
0.0
0.0
0.15

[profilepnt]
4.0
0.0
1.0
0.15

[path]
0
-2.0
0.0
3.5
0

[path]
0
2.0
0.0
3.5
1

[path_2]
1
-5.0
0.2
1.5
2
1

[rail_enh]
200.0
1
30.0
0.02
7.0
30.0
0.02
7.0
"#;

    let sli = SplineProfile::from_text(sli_text).expect("SplineProfile should parse");
    assert_eq!(sli.length, Some(7.5));
    assert_eq!(sli.half_cant_width, Some(1.435));
    assert!(sli.scale_tex_by_length);
    assert_eq!(sli.render_type.as_deref(), Some("presurface"));

    assert_eq!(sli.height_profiles.len(), 3);
    assert_eq!(sli.height_profiles[0].x_start, -6.0);
    assert_eq!(sli.height_profiles[0].x_end, -4.0);
    assert_eq!(sli.height_profiles[1].x_start, -4.0);
    assert_eq!(sli.height_profiles[1].x_end, 4.0);

    assert_eq!(sli.textures.len(), 2);
    assert_eq!(sli.textures[0].file, "curb.bmp");
    assert_eq!(sli.textures[1].file, "asphalt.bmp");

    assert_eq!(sli.graphical_profiles.len(), 2);
    assert_eq!(sli.graphical_profiles[0].texture_index, 0);
    assert_eq!(sli.graphical_profiles[0].points.len(), 2);
    assert_eq!(sli.graphical_profiles[1].texture_index, 1);
    assert_eq!(sli.graphical_profiles[1].points.len(), 2);

    assert_eq!(sli.paths.len(), 3);
    assert_eq!(sli.paths[0].path_type, 0);
    assert_eq!(sli.paths[0].offset_x, -2.0);
    assert_eq!(sli.paths[0].direction, 0);
    assert_eq!(sli.paths[1].path_type, 0);
    assert_eq!(sli.paths[1].offset_x, 2.0);
    assert_eq!(sli.paths[1].direction, 1);
    assert_eq!(sli.paths[2].path_type, 1);
    assert_eq!(sli.paths[2].offset_x, -5.0);
    assert_eq!(sli.paths[2].direction, 2);
    assert_eq!(sli.paths[2].extra, Some(1));

    let rail = sli.rail_enh.expect("rail_enh should exist");
    assert_eq!(rail.rail_length, 200.0);
    assert_eq!(rail.joints, 1);
    assert_eq!(rail.wavelength, 30.0);
    assert_eq!(rail.amplitude, 0.02);
    assert_eq!(rail.power, 7.0);
}
