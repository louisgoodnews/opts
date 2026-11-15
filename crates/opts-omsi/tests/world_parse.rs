//! Integration and robustness tests for world and scenery parsers (M-0015).
//!
//! Validates:
//! - `.sco` scenery objects (`opts_omsi::formats::scenery::sco`)
//! - `.sli` spline extrusion profiles (`opts_omsi::formats::scenery::sli`)
//! - `global.cfg` map manifests (`opts_omsi::formats::map::global`)
//! - `tile_*.map` map tiles (`opts_omsi::formats::map::tile`)
//! - `.map.terrain` binary height grids (`opts_omsi::formats::map::terrain`)
//! - `.map.water` binary water levels (`opts_omsi::formats::map::water`)

#[path = "world_parse/manifest.rs"]
mod manifest;
#[path = "world_parse/scenery.rs"]
mod scenery;
#[path = "world_parse/spline.rs"]
mod spline;

use opts_omsi::formats::map::{MapManifest, MapTerrain, MapTile, MapWater, RuleKind, RuleOwner};
use opts_omsi::formats::scenery::{SceneryObject, SplineProfile};

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
