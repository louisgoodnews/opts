//! A minimal version 14 tile: an object and a spline, with the position order of the fields.

use super::super::*;

#[test]
fn test_parse_minimal_tile() {
    let text = r#"
File created with test

[version]
14

[terrain]

[object]
0
Sceneryobjects\Example\house.sco
100001
150.0
120.0
5.0
90.0
0.0
0.0
0

[spline]
0
Splines\Example\road.sli
200001
0
0
10.0
2.5
20.0
180.0
50.0
0.0
0
0
0
0
0.0
0.0
0
"#;
    let tile = MapTile::from_text(text).unwrap();
    assert_eq!(tile.version, Some(14));
    assert!(tile.has_terrain);

    assert_eq!(tile.objects.len(), 1);
    let obj = &tile.objects[0];
    assert_eq!(obj.sco_file, r"Sceneryobjects\Example\house.sco");
    assert_eq!(obj.id_code, 100001);
    assert_eq!(obj.position, Vec3::new(150.0, 120.0, 5.0));
    assert_eq!(obj.heading_deg, 90.0);
    assert_eq!(obj.pitch_deg, Some(0.0));
    assert_eq!(obj.roll_deg, Some(0.0));

    assert_eq!(tile.splines.len(), 1);
    let spl = &tile.splines[0];
    assert_eq!(spl.sli_file, r"Splines\Example\road.sli");
    assert_eq!(spl.id_code, 200001);
    // Position order: Line 5 (10.0) is X, Line 6 (2.5) is Z (elevation), Line 7 (20.0) is Y!
    assert_eq!(spl.position, Vec3::new(10.0, 20.0, 2.5));
    assert_eq!(spl.heading_deg, 180.0);
    assert_eq!(spl.length_m, 50.0);
    assert_eq!(spl.radius_m, 0.0);
}
