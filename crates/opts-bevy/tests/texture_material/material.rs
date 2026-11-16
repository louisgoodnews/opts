//! Material bridges and the roughness mapping.

use super::*;

#[test]
fn test_o3d_and_x_material_bridge() {
    let o3d_mat = O3dMaterial {
        diffuse: [1.0, 0.5, 0.0, 1.0],
        specular: [0.5, 0.5, 0.5],
        emissive: [0.2, 0.0, 0.0],
        power: 20.0,
        textures: vec!["exterior.dds".into()],
    };

    let std_mat: StandardMaterial = o3d_material_to_standard_material(&o3d_mat, None);
    assert_eq!(std_mat.alpha_mode, AlphaMode::Opaque);
    assert_eq!(std_mat.metallic, 0.0);
    assert!(std_mat.perceptual_roughness < 1.0);
    assert!(std_mat.perceptual_roughness > 0.0);

    let x_mat = XMaterial {
        name: Some("tinted_window".into()),
        diffuse: [0.1, 0.1, 0.2, 0.5],
        specular: [1.0, 1.0, 1.0],
        emissive: [0.0, 0.0, 0.0],
        power: 100.0,
        texture_filename: Some("glass.tga".into()),
    };

    let std_mat_x: StandardMaterial = x_material_to_standard_material(&x_mat, None);
    assert_eq!(std_mat_x.alpha_mode, AlphaMode::Blend);
    assert!(std_mat_x.perceptual_roughness < 0.2);
}

#[test]
fn test_specular_power_to_roughness() {
    assert_eq!(specular_power_to_roughness(0.0), 1.0);
    assert_eq!(specular_power_to_roughness(-5.0), 1.0);
    let r1 = specular_power_to_roughness(10.0);
    let r2 = specular_power_to_roughness(50.0);
    assert!(r1 > r2); // Higher power -> lower roughness (glossier)
}

#[test]
fn test_opts_material_plugin_app_registration() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    app.add_plugins(OptsMaterialPlugin);

    let mut unified_app = App::new();
    unified_app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    unified_app.add_plugins(OptsAssetPlugin);
    // Verify plugins register without panics
}
