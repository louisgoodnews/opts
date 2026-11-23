//! Keyword-specific readings of `model.cfg`: optional variables, live files and light effects.

use super::*;

#[test]
fn test_matl_lightmap_optional_variable() {
    let text = r#"
[mesh]
test.o3d

[matl]
test1.dds
0

[matl_lightmap]
lights1.dds

[matl]
test2.dds
1

[matl_lightmap]
lights2.dds
light_switch_var
"#;
    let config = ModelConfig::parse(text).expect("Should parse matl_lightmap with 1 or 2 params");
    let mesh = &config.lods[0].meshes[0];
    assert_eq!(mesh.materials.len(), 2);
    let lm1 = mesh.materials[0].light_map.as_ref().unwrap();
    assert_eq!(lm1.texture, "lights1.dds");
    assert_eq!(lm1.variable, "");

    let lm2 = mesh.materials[1].light_map.as_ref().unwrap();
    assert_eq!(lm2.texture, "lights2.dds");
    assert_eq!(lm2.variable, "light_switch_var");
}

#[test]
fn test_citybus_facelift_model_cfg_live() {
    let Some(root) = std::env::var_os("OMSI_PATH") else {
        return;
    };
    let path = std::path::Path::new(&root)
        .join("Vehicles/Citybus 530 Facelift by Kajosoft/Model/model_o530_e3_3.cfg");
    if path.exists() {
        let bytes = std::fs::read(&path).expect("read Citybus model cfg");
        let config =
            ModelConfig::parse_bytes(&bytes).expect("parse Citybus model cfg successfully");
        assert!(!config.lods.is_empty());
        assert!(!config.lods[0].meshes.is_empty());
    }
}

/// A keyword with a blank parameter line takes the blank as its (empty) value; the next text is not
/// its parameter (`model_GN92_trail.cfg`: `[matl_transmap]`, blank lines, `#####`, `Rauchsystem`).
#[test]
fn blank_transmap_value_does_not_swallow_the_next_text() {
    let text = "[mesh]\nGN92\\x.o3d\n\n[matl]\nt.bmp\n0\n\n[matl_transmap]\n\n\n\n########################################\n\n\nRauchsystem\n\n[smoke]\n-1.16\n-3.5\n0.46\n-1\n";
    let model = ModelConfig::parse(text).expect("parses");
    let mesh = &model.lods[0].meshes[0];
    assert_eq!(
        mesh.materials[0].trans_map, None,
        "empty value, not 'Rauchsystem'"
    );
    // a texture name on the next line is still read
    let named = "[mesh]\nx.o3d\n[matl]\nt.bmp\n0\n[matl_transmap]\nmask.bmp\n";
    let model = ModelConfig::parse(named).expect("parses");
    assert_eq!(
        model.lods[0].meshes[0].materials[0].trans_map.as_deref(),
        Some("mask.bmp")
    );
}

/// A `[smoke]` block with fewer than the six position and direction values is a warning on the
/// model; it does not reject the model (the GN92 trailer's short emitter, ISS-038).
#[test]
fn a_short_smoke_block_is_a_warning_and_the_model_still_parses() {
    let text = "[mesh]\nGN92\\x.o3d\n\n[smoke]\n-1.16\n-3.5\n0.46\n-1\n";
    let model = ModelConfig::parse(text).expect("parses");
    assert!(model.smokes.is_empty());
    assert_eq!(model.warnings.len(), 1, "{:?}", model.warnings);
    assert!(model.warnings[0].contains("smoke"), "{:?}", model.warnings);
}

#[test]
fn light_effects_of_a_vehicle_model_parse_with_their_lod_and_mesh() {
    // An AI car as on Berlin-Spandau: a dipped-beam corona bound to `AI_Light` after the body
    // mesh, a brake light, and one malformed block.
    let text = r#"
[LOD]
0.1

[mesh]
body.o3d

[light_enh_2]
-0.6
2.1
0.55
0
1
0
0
0
1
0
0
255
250
220
0.4
30
60
AI_Light
1
0
0
0
0.1

[light_enh]
0.6
-2.2
0.6
255
0
0
0.25
AI_Brakelight

[light_enh_2]
1
2
"#;
    let config = parse_model(text).expect("parses");
    assert_eq!(config.lights.len(), 2);
    let head = &config.lights[0];
    assert_eq!(head.variable, "AI_Light");
    assert!((head.position.y - 2.1).abs() < 1e-6 && !head.omni);
    assert!((head.direction.y - 1.0).abs() < 1e-6);
    assert_eq!((head.cone_inner_deg, head.cone_outer_deg), (30.0, 60.0));
    assert!((head.time_const_s - 0.1).abs() < 1e-6);
    assert_eq!((head.lod_threshold, head.after_mesh), (Some(0.1), Some(0)));
    let brake = &config.lights[1];
    assert_eq!(brake.variable, "AI_Brakelight");
    assert!(brake.omni && brake.color == [255.0, 0.0, 0.0]);
    assert_eq!(config.warnings.len(), 1);
    // The light blocks no longer land among the mesh's unknown keywords.
    assert!(!config.lods[0].meshes[0]
        .unknown_fields
        .contains_key("light_enh_2"));
}
