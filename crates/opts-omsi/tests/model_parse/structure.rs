//! Sample bus structure: meshes, viewpoint flags and the default level of detail.

use super::*;

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
