//! Robustness, live corpus validation, spline attachment, text-texture and spline-helper parser tests.

use super::*;

#[test]
fn test_robustness_pseudorandom_fuzzing() {
    // Generate pseudo-random buffers and verify no panics occur
    let mut state: u64 = 0xDEADBEEFCAFEBABE;
    let mut lcg = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as u8
    };

    for len in [0, 1, 3, 4, 19, 20, 21, 50, 100, 1000] {
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(lcg());
        }

        // None of these parsers should ever panic on garbage input
        let _ = SceneryObject::parse(&buf);
        let _ = SplineProfile::parse(&buf);
        let _ = MapManifest::parse(&buf);
        let _ = MapTile::parse(&buf);
        let _ = MapTerrain::parse(&buf);
        let _ = MapWater::parse(&buf);
    }
}

#[test]
fn test_omsi2_corpus_live_validation() {
    let Some(omsi_root) = std::env::var_os("OMSI_PATH")
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists())
    else {
        eprintln!("OMSI_PATH not set; skipping live corpus verification");
        return;
    };
    let omsi_root = omsi_root.as_path();

    // 1. Verify Grundorf global.cfg
    let grundorf_global = omsi_root.join("maps/Grundorf/global.cfg");
    if grundorf_global.exists() {
        let bytes = std::fs::read(&grundorf_global).expect("read Grundorf global.cfg");
        let manifest = MapManifest::parse(&bytes).expect("parse Grundorf global.cfg");
        assert_eq!(manifest.name, "Grundorf");
        assert!(!manifest.tiles.is_empty());
    }

    // 2. Verify Grundorf tile_0_0.map & terrain
    let grundorf_tile = omsi_root.join("maps/Grundorf/tile_0_0.map");
    if grundorf_tile.exists() {
        let bytes = std::fs::read(&grundorf_tile).expect("read Grundorf tile_0_0.map");
        let tile = MapTile::parse(&bytes).expect("parse Grundorf tile_0_0.map");
        assert!(tile.version.is_some());
        assert!(!tile.objects.is_empty() || !tile.splines.is_empty());
    }

    let grundorf_terrain = omsi_root.join("maps/Grundorf/tile_0_0.map.terrain");
    if grundorf_terrain.exists() {
        let bytes = std::fs::read(&grundorf_terrain).expect("read Grundorf terrain");
        let terrain = MapTerrain::parse(&bytes).expect("parse Grundorf terrain");
        assert_eq!(terrain.grid_cells, 60);
        assert_eq!(terrain.heights.len(), 61 * 61);
    }

    // 3. Verify Berlin-Spandau global.cfg if present
    let spandau_global = omsi_root.join("maps/Berlin-Spandau/global.cfg");
    if spandau_global.exists() {
        let bytes = std::fs::read(&spandau_global).expect("read Spandau global.cfg");
        let manifest = MapManifest::parse(&bytes).expect("parse Spandau global.cfg");
        assert_eq!(manifest.name, "Berlin-Spandau");
        assert!(manifest.tiles.len() > 10);
    }

    // 4. Sample real .sco files
    let sco_dir = omsi_root.join("Sceneryobjects");
    if sco_dir.exists() {
        let mut tested = 0;
        for entry in walkdir(sco_dir, 50, ".sco") {
            let bytes = std::fs::read(&entry).expect("read sco");
            let _ = SceneryObject::parse(&bytes).expect("sco should parse cleanly");
            tested += 1;
        }
        assert!(tested > 0, "should test at least one .sco file");
    }

    // 5. Sample real .sli files
    let sli_dir = omsi_root.join("Splines");
    if sli_dir.exists() {
        let mut tested = 0;
        for entry in walkdir(sli_dir, 50, ".sli") {
            let bytes = std::fs::read(&entry).expect("read sli");
            let _ = SplineProfile::parse(&bytes).expect("sli should parse cleanly");
            tested += 1;
        }
        assert!(tested > 0, "should test at least one .sli file");
    }
}

fn walkdir(dir: std::path::PathBuf, limit: usize, ext: &str) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    let mut stack = vec![dir];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(name) = path.to_str() {
                        if name.ends_with(ext) {
                            results.push(path);
                            if results.len() >= limit {
                                return results;
                            }
                        }
                    }
                }
            }
        }
    }
    results
}

#[test]
fn spline_attachments_parse_single_and_repeater() {
    let text = "[version]\n14\n\n[splineAttachement]\n0\nSceneryobjects\\a.sco\n77\n3\n-4.5\n0.25\n12.5\n180\n0\n0\n30\n500\n0\n0\n\n[splineAttachement_repeater]\n0\n260\n6\nSceneryobjects\\b.sco\n78\n1\n9.5\n0.1\n8.8\n0\n0\n0\n30\n500\n0\n";
    let tile = opts_omsi::formats::map::MapTile::from_text(text).unwrap();
    assert_eq!(tile.spline_attachments.len(), 2);
    let single = &tile.spline_attachments[0];
    assert_eq!(
        (single.spline_index, single.lateral_m, single.distance_m),
        (3, -4.5, 12.5)
    );
    assert_eq!((single.spacing_m, single.range_m), (30.0, 500.0));
    assert_eq!(single.repeater, None);
    assert_eq!(single.instances(), 0..=16);
    let rep = &tile.spline_attachments[1];
    assert_eq!(
        rep.repeater,
        Some(opts_omsi::formats::map::RepeaterLink {
            master_tile: 260,
            first_instance: 6
        })
    );
    assert_eq!(rep.instances(), 6..=16);
    assert_eq!(
        (rep.spline_index, rep.sco_file.as_str()),
        (1, "Sceneryobjects\\b.sco")
    );
}

#[test]
fn text_texture_binds_to_the_preceding_material() {
    let text = "[texttexture_enh]\n1\nStreetBerlin\n256\n32\n0\n10\n20\n30\n2\n0\n\n[mesh]\nplate.o3d\n\n[matl]\nTextfeld_1.bmp\n0\n\n[useTextTexture]\n0\n\n[matl_alpha]\n2\n\n[matl]\nother.bmp\n1\n";
    let sco = SceneryObject::from_text(text).expect("parse");
    let tt = &sco.text_textures[0];
    assert_eq!(
        (tt.string_index, tt.orientation, tt.text_color),
        (1, 2, [10, 20, 30])
    );
    let mats = &sco.meshes[0].materials;
    assert_eq!(mats[0].text_texture, Some(0));
    assert_eq!(mats[0].alpha_mode, Some(2));
    assert_eq!(mats[1].text_texture, None);
}

#[test]
fn spline_helper_lines_are_x_height_y_and_crossings_are_absolute() {
    // Lines after the profile: x, height, y, heading, two further values (Einmuedung_Gewerbegebiet.sco).
    let text = "[splinehelper]\nSplines\\a.sli\n-12\n0.5\n-3\n270\n0\n0\n\n[mesh]\nm.o3d\n";
    let sco = SceneryObject::from_text(text).expect("parse");
    let h = &sco.spline_helpers[0];
    assert_eq!(
        (h.position.x, h.position.y, h.position.z),
        (-12.0, -3.0, 0.5)
    );
    assert_eq!(h.rotation_deg, 270.0);
    assert!(!sco.abs_height, "no [absheight]");
    assert!(
        sco.has_absolute_height(),
        "a crossing is stored at absolute height"
    );
    let plain = SceneryObject::from_text("[mesh]\nm.o3d\n").expect("parse");
    assert!(!plain.has_absolute_height());
}
