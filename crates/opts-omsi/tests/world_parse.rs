//! Integration and robustness tests for world and scenery parsers (M-0015).
//!
//! Validates:
//! - `.sco` scenery objects (`opts_omsi::formats::scenery::sco`)
//! - `.sli` spline extrusion profiles (`opts_omsi::formats::scenery::sli`)
//! - `global.cfg` map manifests (`opts_omsi::formats::map::global`)
//! - `tile_*.map` map tiles (`opts_omsi::formats::map::tile`)
//! - `.map.terrain` binary height grids (`opts_omsi::formats::map::terrain`)
//! - `.map.water` binary water levels (`opts_omsi::formats::map::water`)

use glam::Vec3;
use opts_omsi::formats::map::{MapManifest, MapTerrain, MapTile, MapWater, RuleKind, RuleOwner};
use opts_omsi::formats::scenery::{SceneryObject, SplineProfile};

#[test]
fn test_scenery_object_full_feature_parsing() {
    let sco_text = r#"
[groups]
3
Berlin-Spandau
Streetobjects
Signs

[friendlyname]
Haltestelle_Mast_Standard

[complexity]
1

[fixed]
[surface]
[absheight]

[mesh]
Mast.o3d

[shadow]

[matl]
mast_tex.bmp
0

[matl_alpha]
1

[matl_envmap]
envmap.bmp
0.25

[matl_lightmap]
mast_lm.bmp

[mesh]
Schild.o3d

[visible]
show_sign
1

[usetexttexture]
0

[boundingbox]
0.4
0.4
3.0
0.0
0.0
1.5

[cog]
0.0
0.0
1.0

[crashmode_pole]
0.05
0.7

[texttexture]
0
DIN
256
64
0
255
255
255

[ctc]
skin_var
Texture\Skins
0

[ctctexture]
BVG
Texture\Skins\bvg.bmp

[script]
1
script\sign.osc

[varnamelist]
1
script\sign_varlist.txt

[stringvarnamelist]
1
script\sign_strlist.txt

[constfile]
1
script\sign_const.txt

[sound]
sound\sign.cfg

[passengercabin]
model\passengercabin.cfg
"#;

    let sco = SceneryObject::from_text(sco_text).expect("SceneryObject should parse");
    assert_eq!(sco.groups, vec!["Berlin-Spandau", "Streetobjects", "Signs"]);
    assert_eq!(
        sco.friendly_name.as_deref(),
        Some("Haltestelle_Mast_Standard")
    );
    assert_eq!(sco.complexity, Some(1));
    assert!(sco.fixed);
    assert!(sco.surface);
    assert!(sco.abs_height);

    assert_eq!(sco.meshes.len(), 2);
    assert_eq!(sco.meshes[0].file, "Mast.o3d");
    assert_eq!(sco.meshes[0].cast_shadow, Some(true));
    assert_eq!(sco.meshes[0].materials.len(), 1);
    assert_eq!(sco.meshes[0].materials[0].texture, "mast_tex.bmp");
    assert_eq!(sco.meshes[0].materials[0].alpha_mode, Some(1));
    assert_eq!(
        sco.meshes[0].materials[0].env_map,
        Some(("envmap.bmp".to_string(), 0.25))
    );
    assert_eq!(
        sco.meshes[0].materials[0].light_map.as_deref(),
        Some("mast_lm.bmp")
    );

    assert_eq!(sco.meshes[1].file, "Schild.o3d");
    assert_eq!(sco.meshes[1].visible, Some(("show_sign".to_string(), 1.0)));
    assert_eq!(sco.meshes[1].text_texture_indices, vec![0]);

    let bbox = sco.bounding_box.expect("bounding box should exist");
    assert_eq!(bbox.half_extents, Vec3::new(0.4, 0.4, 3.0));
    assert_eq!(bbox.center, Vec3::new(0.0, 0.0, 1.5));
    assert_eq!(sco.cog, Some(Vec3::new(0.0, 0.0, 1.0)));
    assert_eq!(sco.crashmode_pole, Some((0.05, 0.7)));

    assert_eq!(sco.text_textures.len(), 1);
    assert_eq!(sco.text_textures[0].string_index, 0);
    assert_eq!(sco.text_textures[0].font_name, "DIN");
    assert_eq!(sco.text_textures[0].width, 256);
    assert_eq!(sco.text_textures[0].height, 64);
    assert_eq!(sco.text_textures[0].text_color, [255, 255, 255]);

    let ctc = sco.ctc.expect("CTC should exist");
    assert_eq!(ctc.var_name, "skin_var");
    assert_eq!(ctc.texture_dir, r"Texture\Skins");
    assert_eq!(ctc.default_index, 0);
    assert_eq!(
        sco.ctc_textures,
        vec![("BVG".to_string(), r"Texture\Skins\bvg.bmp".to_string())]
    );

    assert_eq!(sco.scripts, vec![r"script\sign.osc"]);
    assert_eq!(sco.varnamelists, vec![r"script\sign_varlist.txt"]);
    assert_eq!(sco.stringvarnamelists, vec![r"script\sign_strlist.txt"]);
    assert_eq!(sco.constfiles, vec![r"script\sign_const.txt"]);
    assert_eq!(sco.sound.as_deref(), Some(r"sound\sign.cfg"));
    assert_eq!(
        sco.passenger_cabin.as_deref(),
        Some(r"model\passengercabin.cfg")
    );
}

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

#[test]
fn rules_follow_the_element_before_them_and_unknown_types_have_no_kind() {
    let text = "[version]\n14\n\n[rule]\n0\ntrafficdensity\n0\n0\n\n[object]\n0\nA.sco\n7\n1\n2\n3\n0\n0\n0\n0\n\n[rule]\n2\nSpeedLimit\n30\n0\n\n[rule]\n0\nfuture_rule\n1\n0\n";
    let tile = MapTile::from_text(text).expect("parses");
    assert_eq!(tile.rules.len(), 3);
    assert_eq!(tile.rules[0].owner, None);
    assert_eq!(
        tile.rules[1].owner,
        Some(RuleOwner::Object {
            index: 0,
            id_code: 7
        })
    );
    assert_eq!(tile.rules[1].kind(), Some(RuleKind::SpeedLimit));
    assert_eq!(tile.rules[2].kind(), None);
}

#[test]
fn test_binary_terrain_and_water_parsing() {
    let mut terrain_data = Vec::with_capacity(MapTerrain::STANDARD_FILE_SIZE);
    terrain_data.extend_from_slice(&60u32.to_le_bytes());
    for i in 0..(61 * 61) {
        let h = (i as f32) * 0.05 + 10.0;
        terrain_data.extend_from_slice(&h.to_le_bytes());
    }
    let terrain = MapTerrain::parse(&terrain_data).expect("terrain should parse");
    assert_eq!(terrain.grid_cells, 60);
    assert_eq!(terrain.get_height(0, 0), Some(10.0));
    assert_eq!(terrain.get_height(60, 60), Some(10.0 + 3720.0 * 0.05));
    assert_eq!(terrain.get_height(61, 0), None);

    let mut water_data = Vec::with_capacity(20);
    water_data.extend_from_slice(&1u32.to_le_bytes());
    water_data.extend_from_slice(&0.0f32.to_le_bytes());
    water_data.extend_from_slice(&0.0f32.to_le_bytes());
    water_data.extend_from_slice(&0.0f32.to_le_bytes());
    water_data.extend_from_slice(&0.0f32.to_le_bytes());
    let water = MapWater::parse(&water_data).expect("water should parse");
    assert_eq!(water.version, 1);
    assert_eq!(water.levels, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_utf16le_bom_encoding_in_map_files() {
    let raw_ascii = "[version]\r\n14\r\n[name]\r\nTestMap\r\n";
    let mut u16_bytes = vec![0xFF, 0xFE]; // UTF-16LE BOM
    for c in raw_ascii.encode_utf16() {
        u16_bytes.extend_from_slice(&c.to_le_bytes());
    }

    let manifest = MapManifest::parse(&u16_bytes).expect("UTF-16LE manifest should parse");
    assert_eq!(manifest.version, Some(14));
    assert_eq!(manifest.name, "TestMap");
}

#[path = "world_parse/corpus.rs"]
mod corpus;
