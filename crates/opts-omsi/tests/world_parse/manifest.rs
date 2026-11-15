//! `global.cfg` map manifests and map tiles of the world parse tests (`opts_omsi::formats::map`).

use glam::Vec3;
use opts_omsi::formats::map::{MapManifest, MapTile, RuleKind, RuleOwner};

#[test]
fn test_map_manifest_full_feature_parsing() {
    let global_text = r#"
[name]
Berlin-Spandau_1989

[friendlyname]
Berlin-Spandau (1989)

[description]
Historical Berlin Spandau during the fall of the wall.
Line 92 from Falkensee to Freudstraße.
[end]

[version]
14

[NextIDCode]
499120

[moneysystem]
Money\DM\DM.cfg

[ticketpack]
Ticketpacks\Berlin_1989\Berlin_1989.otp

[standarddepot]
Spandau

[years]
1986
1994

[repair_time_min]
15.0

[worldcoordinates]
[realrail]

[map]
2402
11279
tile_2402_11279.map

[map]
2402
11280
tile_2402_11280.map

[groundtex]
str_asphalt.bmp
str_detail.bmp
0.0
1.0
60.0

[addseason]
1
60
150

[addseason]
3
330
60

[trafficdensity_road]
8.0
1.5

[trafficdensity_passenger]
8.0
2.0
"#;

    let manifest = MapManifest::from_text(global_text).expect("MapManifest should parse");
    assert_eq!(manifest.name, "Berlin-Spandau_1989");
    assert_eq!(
        manifest.friendly_name.as_deref(),
        Some("Berlin-Spandau (1989)")
    );
    assert_eq!(manifest.description.len(), 2);
    assert_eq!(manifest.version, Some(14));
    assert_eq!(manifest.next_id_code, Some(499120));
    assert_eq!(manifest.money_system.as_deref(), Some(r"Money\DM\DM.cfg"));
    assert_eq!(manifest.years, Some((1986, 1994)));
    assert_eq!(manifest.repair_time_min, Some(15.0));
    assert!(manifest.world_coordinates);
    assert!(manifest.real_rail);

    assert_eq!(manifest.tiles.len(), 2);
    assert_eq!(manifest.tiles[0].x, 2402);
    assert_eq!(manifest.tiles[0].y, 11279);
    assert_eq!(manifest.tiles[0].file_name, "tile_2402_11279.map");

    assert_eq!(manifest.ground_textures.len(), 1);
    assert_eq!(
        manifest.ground_textures[0].ground_texture,
        "str_asphalt.bmp"
    );
    assert_eq!(manifest.ground_textures[0].params, [0.0, 1.0, 60.0]);

    assert_eq!(manifest.seasons.len(), 2);
    assert_eq!(manifest.seasons[0].season_id, 1);
    assert_eq!(manifest.seasons[0].start_day, 60);
    assert_eq!(manifest.seasons[0].end_day, 150);

    assert_eq!(manifest.traffic_density_road, vec![(8.0, 1.5)]);
    assert_eq!(manifest.traffic_density_passenger, vec![(8.0, 2.0)]);
}

#[test]
fn test_map_tile_and_coordinate_conventions() {
    let tile_text = r#"
File created with OMSI-Editor

[version]
14

[terrain]
[water]

[object]
0
Sceneryobjects\Generic\lamp.sco
5501
120.5
240.8
1.2
180.0
0.0
0.0
2
Lamp_Skin_Green.bmp
12.5

[spline]
0
Splines\Generic\2lane_8m.sli
9901
0
0
50.0
3.5
75.0
90.0
120.0
0.0
1.0
1.0
0
0
0
0
0

[rule]
0
speedlimit
50.0
0

[rule]
1
priority
128.0
0
"#;

    let tile = MapTile::from_text(tile_text).expect("MapTile should parse");
    assert_eq!(tile.version, Some(14));
    assert!(tile.has_terrain);
    assert!(tile.has_water);

    assert_eq!(tile.objects.len(), 1);
    let obj = &tile.objects[0];
    assert_eq!(obj.sco_file, r"Sceneryobjects\Generic\lamp.sco");
    assert_eq!(obj.id_code, 5501);
    assert_eq!(obj.position, Vec3::new(120.5, 240.8, 1.2));
    assert_eq!(obj.heading_deg, 180.0);
    assert_eq!(obj.pitch_deg, Some(0.0));
    assert_eq!(obj.roll_deg, Some(0.0));
    assert_eq!(obj.string_vars, vec!["Lamp_Skin_Green.bmp", "12.5"]);

    assert_eq!(tile.splines.len(), 1);
    let spl = &tile.splines[0];
    assert_eq!(spl.sli_file, r"Splines\Generic\2lane_8m.sli");
    assert_eq!(spl.id_code, 9901);
    // Line 5: X = 50.0
    // Line 6: Z (elevation) = 3.5
    // Line 7: Y = 75.0
    assert_eq!(spl.position, Vec3::new(50.0, 75.0, 3.5));
    assert_eq!(spl.heading_deg, 90.0);
    assert_eq!(spl.length_m, 120.0);
    assert_eq!(spl.radius_m, 0.0);
    assert_eq!(spl.gradient_start, 0.01);
    assert_eq!(spl.gradient_end, 0.01);

    assert_eq!(tile.rules.len(), 2);
    assert_eq!(tile.rules[0].path_index, 0);
    assert_eq!(tile.rules[0].rule_type, "speedlimit");
    assert_eq!(tile.rules[0].value, 50.0);
    assert_eq!(tile.rules[1].path_index, 1);
    assert_eq!(tile.rules[1].rule_type, "priority");
    assert_eq!(tile.rules[1].value, 128.0);
    // Both rules follow the spline (the last placed element), not the earlier object.
    let owner = Some(RuleOwner::Spline {
        index: 0,
        id_code: 9901,
    });
    assert_eq!(tile.rules[0].owner, owner);
    assert_eq!(tile.rules[1].owner, owner);
    assert_eq!(tile.rules[0].kind(), Some(RuleKind::SpeedLimit));
    assert_eq!(tile.rules[1].kind(), Some(RuleKind::Priority));
}
