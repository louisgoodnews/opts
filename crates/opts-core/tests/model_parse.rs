//! Integration and fuzz tests for vehicle model configuration parsing (`model.cfg`).

use opts_core::formats::vehicles::model::{
    parse_model, ModelConfig, ModelParseError, ViewpointFlags,
};

const SAMPLE_BUS_MODEL: &str = r#"
[tex_detail_factor]
0.5

[CTC]
Colorscheme
Texture\Werbung_GN92
0

[CTCTexture]
farbschema_tex1
GN92_01.bmp

[texttexture]
number
Kennz_DtAlt
128
64
0
0
0
0

[texttexture_enh]
matrix
Linien_Matrix
512
128
1
255
180
0
0
1

[scripttexture]
256
256

[spotlight]
0.0
5.2
0.65
0.0
1.0
-0.05
255
255
240
150.0
12.0
25.0

[LOD]
0.1

[mesh]
Generic\GN_Body_Front.o3d

[viewpoint]
1

[illumination_interior]
0
1
-1
-1

[mesh_ident]
body_main

[isshadow]

[matl]
GN92_Exterior.bmp
0

[matl_alpha]
2

[matl_envmap]
envmap_body.bmp
0.35

[matl_change]
GN92_Exterior_Alt.bmp
0
cp_repaint_flag

[mesh]
Generic\GN_Steering_Col.o3d

[viewpoint]
2

[mesh_ident]
steering_column

[mouseevent]
cp_horn_button

[visible]
dashboard_active
1.0

[newanim]
origin_trans
-0.65
4.85
1.15
origin_rot_x
-15.0
anim_trans
steering_adj
0.05
maxspeed
1.5

[mesh]
Generic\GN_Steering_Wheel.o3d

[viewpoint]
2

[animparent]
steering_column

[newanim]
origin_from_mesh
origin_rot_z
-15.0
anim_rot
steering_wheel_rot
540.0
delay
0.8
maxspeed
4.0

[interiorlight]
lights_cab
0.7
205
200
175
-0.65
4.85
1.85

[mesh]
Generic\GN_Bellows.o3d

[viewpoint]
3

[smoothskin]

[setbone]
bellows_front
0

[setbone]
bellows_rear
1

[LOD]
0.0

[mesh]
Generic\GN_Body_Lowpoly.o3d

[viewpoint]
4
"#;

#[test]
fn test_parse_sample_bus_model() {
    let config = ModelConfig::parse(SAMPLE_BUS_MODEL).expect("Failed to parse valid bus model.cfg");

    // Global properties
    assert_eq!(config.tex_detail_factor, Some(0.5));
    assert_eq!(config.ctc_schemes.len(), 1);
    assert_eq!(config.ctc_schemes[0].name, "Colorscheme");
    assert_eq!(config.ctc_textures.len(), 1);
    assert_eq!(config.ctc_textures[0].name, "farbschema_tex1");
    assert_eq!(config.text_textures.len(), 2);
    assert_eq!(config.text_textures[0].variable, "number");
    assert_eq!(config.text_textures[1].variable, "matrix");
    assert_eq!(config.text_textures[1].orientation, Some(0));
    assert_eq!(config.text_textures[1].grid, Some(1));
    assert_eq!(config.script_textures.len(), 1);
    assert_eq!(config.script_textures[0].width, 256);

    // Global spotlight
    assert_eq!(config.spotlights.len(), 1);
    assert_eq!(config.spotlights[0].range, 150.0);

    // LOD tiers
    assert_eq!(config.lods.len(), 2);
    assert_eq!(config.lods[0].threshold, 0.1);
    assert_eq!(config.lods[0].meshes.len(), 4);
    assert_eq!(config.lods[1].threshold, 0.0);
    assert_eq!(config.lods[1].meshes.len(), 1);
    assert_eq!(config.total_mesh_count(), 5);

    // Mesh 0: Exterior body
    let mesh0 = &config.lods[0].meshes[0];
    assert_eq!(mesh0.path, r"Generic\GN_Body_Front.o3d");
    assert!(mesh0.viewpoint.is_outside());
    assert!(!mesh0.viewpoint.is_inside());
    assert_eq!(mesh0.illumination_interior, Some([0, 1, -1, -1]));
    assert_eq!(mesh0.mesh_ident.as_deref(), Some("body_main"));
    assert!(mesh0.isshadow);
    assert_eq!(mesh0.materials.len(), 1);
    assert_eq!(mesh0.materials[0].texture, "GN92_Exterior.bmp");
    assert_eq!(mesh0.materials[0].alpha_mode, Some(2));
    assert_eq!(
        mesh0.materials[0].env_map.as_ref().map(|e| e.factor),
        Some(0.35)
    );
    assert_eq!(mesh0.materials[0].texture_changes.len(), 1);
    assert_eq!(
        mesh0.materials[0].texture_changes[0].variable,
        "cp_repaint_flag"
    );

    // Mesh 1: Steering column
    let mesh1 = &config.lods[0].meshes[1];
    assert_eq!(mesh1.path, r"Generic\GN_Steering_Col.o3d");
    assert!(mesh1.viewpoint.is_inside());
    assert!(!mesh1.viewpoint.is_outside());
    assert_eq!(mesh1.mouseevent.as_deref(), Some("cp_horn_button"));
    assert_eq!(mesh1.visibility.len(), 1);
    assert_eq!(mesh1.visibility[0].variable, "dashboard_active");
    assert_eq!(mesh1.anims.len(), 1);
    assert!(mesh1.anims[0].is_translation());
    assert_eq!(mesh1.anims[0].driver_variable(), Some("steering_adj"));

    // Mesh 2: Steering wheel
    let mesh2 = &config.lods[0].meshes[2];
    assert_eq!(mesh2.path, r"Generic\GN_Steering_Wheel.o3d");
    assert_eq!(mesh2.animparent.as_deref(), Some("steering_column"));
    assert_eq!(mesh2.anims.len(), 1);
    assert!(mesh2.anims[0].origin_from_mesh);
    assert!(mesh2.anims[0].is_rotation());
    assert_eq!(mesh2.anims[0].driver_variable(), Some("steering_wheel_rot"));
    assert_eq!(mesh2.anims[0].delay, Some(0.8));
    assert_eq!(mesh2.anims[0].maxspeed, Some(4.0));
    assert_eq!(mesh2.interior_lights.len(), 1);
    assert_eq!(mesh2.interior_lights[0].variable, "lights_cab");

    // Mesh 3: Skinned articulation bellows
    let mesh3 = &config.lods[0].meshes[3];
    assert_eq!(mesh3.path, r"Generic\GN_Bellows.o3d");
    assert!(mesh3.smoothskin);
    assert_eq!(mesh3.bones.len(), 2);
    assert_eq!(mesh3.bones[0].name, "bellows_front");
    assert_eq!(mesh3.bones[1].name, "bellows_rear");

    // LOD 1 Mesh 0: AI lowpoly
    let mesh_low = &config.lods[1].meshes[0];
    assert_eq!(mesh_low.path, r"Generic\GN_Body_Lowpoly.o3d");
    assert!(mesh_low.viewpoint.is_ai());
}

#[test]
fn test_viewpoint_flag_combinations() {
    let always = ViewpointFlags::ALWAYS;
    assert!(always.is_always());
    assert!(always.is_outside());
    assert!(always.is_inside());
    assert!(always.is_ai());

    let outside = ViewpointFlags::OUTSIDE;
    assert!(!outside.is_always());
    assert!(outside.is_outside());
    assert!(!outside.is_inside());
    assert!(!outside.is_ai());

    let inside = ViewpointFlags::INSIDE;
    assert!(!inside.is_always());
    assert!(!inside.is_outside());
    assert!(inside.is_inside());
    assert!(!inside.is_ai());

    let both = ViewpointFlags::new(3);
    assert!(!both.is_always());
    assert!(both.is_outside());
    assert!(both.is_inside());
    assert!(!both.is_ai());

    let ai = ViewpointFlags::AI_BUS;
    assert!(!ai.is_always());
    assert!(!ai.is_outside());
    assert!(!ai.is_inside());
    assert!(ai.is_ai());
}

#[test]
fn test_default_lod_for_unadorned_mesh() {
    let text = r#"
[mesh]
model.o3d
"#;
    let config = parse_model(text).expect("Should parse single mesh");
    assert_eq!(config.lods.len(), 1);
    assert_eq!(config.lods[0].threshold, 0.0);
    assert_eq!(config.lods[0].meshes.len(), 1);
    assert_eq!(config.lods[0].meshes[0].path, "model.o3d");
}

#[test]
fn test_missing_parameters_error() {
    let text = r#"
[mesh]
"#;
    let err = ModelConfig::parse(text).unwrap_err();
    assert!(matches!(err, ModelParseError::MissingParameters { .. }));
}

#[test]
fn test_invalid_number_error() {
    let text = r#"
[LOD]
not_a_number
"#;
    let err = ModelConfig::parse(text).unwrap_err();
    assert!(matches!(err, ModelParseError::InvalidNumber { .. }));
}

#[test]
fn test_unknown_keywords_preserved() {
    let text = r#"
[custom_global]
param_a
param_b

[mesh]
test.o3d

[custom_mesh_prop]
val1
val2
"#;
    let config = ModelConfig::parse(text).expect("Should preserve unknown keywords");
    assert!(config.unknown_fields.contains_key("custom_global"));
    let mesh = &config.lods[0].meshes[0];
    assert!(mesh.unknown_fields.contains_key("custom_mesh_prop"));
}

#[test]
fn test_fuzz_pseudo_random_buffers() {
    let mut state: u32 = 0xDEADBEEF;
    let mut next_u8 = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        (state >> 24) as u8
    };

    for _ in 0..1000 {
        let len = (next_u8() as usize) % 256;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(next_u8());
        }

        // Fuzzing with ModelConfig::parse_bytes must never panic
        let _ = ModelConfig::parse_bytes(&buf);
    }
}

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
    let path = std::path::Path::new("/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/Vehicles/Citybus 530 Facelift by Kajosoft/Model/model_o530_e3_3.cfg");
    if path.exists() {
        let bytes = std::fs::read(path).expect("read Citybus model cfg");
        let config =
            ModelConfig::parse_bytes(&bytes).expect("parse Citybus model cfg successfully");
        assert!(!config.lods.is_empty());
        assert!(!config.lods[0].meshes.is_empty());
    }
}
