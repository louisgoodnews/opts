//! Tests of the material conversion: emission, alpha and the mapping of `.o3d` materials.

use super::*;

#[test]
fn a_self_illuminating_material_glows_with_its_own_texture() {
    let tex: Handle<Image> = Handle::default();
    let lit = LinearRgba::new(1.0, 1.0, 1.0, 1.0);
    assert_eq!(emission_texture(lit, Some(&tex)), Some(tex.clone()));
    assert_eq!(
        emission_texture(LinearRgba::BLACK, Some(&tex)),
        None,
        "no glow, no texture"
    );
    assert_eq!(
        emission_texture(lit, None),
        None,
        "untextured: the colour alone"
    );
    let mat = O3dMaterial {
        diffuse: [0.64, 0.64, 0.64, 1.0],
        specular: [0.0; 3],
        emissive: [1.0; 3],
        power: 0.0,
        textures: Vec::new(),
    };
    let m = o3d_material_to_standard_material(&mat, Some(tex.clone()));
    assert_eq!(m.emissive_texture, Some(tex));
}

#[test]
fn zero_diffuse_alpha_is_opaque_and_fractions_stay() {
    assert_eq!(effective_diffuse_alpha(0.0), 1.0);
    assert_eq!(effective_diffuse_alpha(0.4), 0.4);
    assert_eq!(effective_diffuse_alpha(1.0), 1.0);
}

#[test]
fn test_roughness_conversion() {
    assert_eq!(specular_power_to_roughness(0.0), 1.0);
    let rough_glossy = specular_power_to_roughness(50.0);
    assert!(rough_glossy < 0.5);
    assert!(rough_glossy >= 0.08);
}

#[test]
fn test_o3d_material_conversion() {
    let mat = O3dMaterial {
        diffuse: [0.8, 0.2, 0.2, 1.0],
        specular: [1.0, 1.0, 1.0],
        emissive: [0.1, 0.1, 0.1],
        power: 16.0,
        textures: vec!["bus.dds".into()],
    };

    let std_mat = o3d_material_to_standard_material(&mat, None);
    assert_eq!(std_mat.alpha_mode, AlphaMode::Opaque);
    assert_eq!(std_mat.metallic, 0.0);
    assert!(std_mat.perceptual_roughness < 1.0);
}

#[test]
fn test_translucent_material_uses_blend() {
    let mat = XMaterial {
        name: Some("glass".into()),
        diffuse: [1.0, 1.0, 1.0, 0.4],
        power: 32.0,
        specular: [1.0, 1.0, 1.0],
        emissive: [0.0, 0.0, 0.0],
        texture_filename: None,
    };

    let std_mat = x_material_to_standard_material(&mat, None);
    assert_eq!(std_mat.alpha_mode, AlphaMode::Blend);
}

#[test]
fn alpha_mode_zero_is_opaque() {
    assert!(!uses_texture_alpha(None));
    assert!(!uses_texture_alpha(Some(0)));
    assert!(uses_texture_alpha(Some(1)));
    assert!(uses_texture_alpha(Some(2)));
    assert!(uses_texture_alpha(Some(3)));
}
