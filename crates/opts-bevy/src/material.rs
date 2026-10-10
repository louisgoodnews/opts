//! Material conversion bridge mapping OMSI `.o3d` and DirectX `.x` materials into Bevy [`StandardMaterial`] assets.
//!
//! OMSI materials follow the Direct3D fixed-function lighting model (diffuse RGBA,
//! specular RGB with power exponent, emissive self-illumination RGB, and texture bindings).
//! This module converts these definitions into modern physically-based Bevy [`StandardMaterial`]
//! instances, deriving appropriate base color, roughness, emissive illumination, and
//! transparency modes ([`AlphaMode`]).

#![deny(unsafe_code)]

use bevy::asset::Handle;
use bevy::color::{Color, LinearRgba};
use bevy::image::Image;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;

use opts_core::formats::o3d::Material as O3dMaterial;
use opts_core::formats::x::XMaterial;

/// Converts a Blinn-Phong specular power exponent to PBR perceptual roughness.
///
/// Direct3D models use shininess exponents where higher values imply glossier/smoother surfaces.
/// The conversion uses the standard approximation: $\text{roughness} = \sqrt{2 / (\text{power} + 2)}$.
#[must_use]
pub fn specular_power_to_roughness(power: f32) -> f32 {
    if power <= 0.0 {
        1.0
    } else {
        (2.0 / (power + 2.0)).sqrt().clamp(0.08, 1.0)
    }
}

/// Converts an OMSI `.o3d` [`Material`](O3dMaterial) into a Bevy [`StandardMaterial`].
///
/// # Arguments
/// * `mat` - The parsed `.o3d` material parameters.
/// * `texture_handle` - Optional asset handle to the resolved diffuse texture.
#[must_use]
pub fn o3d_material_to_standard_material(
    mat: &O3dMaterial,
    texture_handle: Option<Handle<Image>>,
) -> StandardMaterial {
    let [r, g, b, a] = mat.diffuse;
    let base_color = Color::srgba(
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    );

    let [er, eg, eb] = mat.emissive;
    let emissive = LinearRgba::new(er.max(0.0), eg.max(0.0), eb.max(0.0), 1.0);

    let perceptual_roughness = specular_power_to_roughness(mat.power);

    let alpha_mode = if a < 0.999 {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };

    StandardMaterial {
        base_color,
        base_color_texture: texture_handle,
        emissive,
        perceptual_roughness,
        metallic: 0.0,
        alpha_mode,
        ..Default::default()
    }
}

/// Converts a Microsoft DirectX `.x` [`XMaterial`] into a Bevy [`StandardMaterial`].
///
/// # Arguments
/// * `mat` - The parsed DirectX `.x` material parameters.
/// * `texture_handle` - Optional asset handle to the resolved diffuse texture.
#[must_use]
pub fn x_material_to_standard_material(
    mat: &XMaterial,
    texture_handle: Option<Handle<Image>>,
) -> StandardMaterial {
    let [r, g, b, a] = mat.diffuse;
    let base_color = Color::srgba(
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    );

    let [er, eg, eb] = mat.emissive;
    let emissive = LinearRgba::new(er.max(0.0), eg.max(0.0), eb.max(0.0), 1.0);

    let perceptual_roughness = specular_power_to_roughness(mat.power);

    let alpha_mode = if a < 0.999 {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };

    StandardMaterial {
        base_color,
        base_color_texture: texture_handle,
        emissive,
        perceptual_roughness,
        metallic: 0.0,
        alpha_mode,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
