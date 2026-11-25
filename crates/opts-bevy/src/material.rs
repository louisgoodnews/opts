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

/// Bevy's `reflectance` (0.5 is a plain dielectric, F0 0.04) for a Direct3D specular colour: the
/// brightness of the colour, halved, so an OMSI material that says no specular colour (roads,
/// pavements, most buildings) has no sun glare and one that says full white keeps the default
/// look. Ignoring the colour made every asphalt shine like a wet road in low sun.
#[must_use]
pub fn specular_to_reflectance(specular: [f32; 3]) -> f32 {
    let lum = 0.2126 * specular[0] + 0.7152 * specular[1] + 0.0722 * specular[2];
    (lum.clamp(0.0, 1.0) * 0.5).clamp(0.0, 0.5)
}

/// The opacity an `.o3d` material's diffuse alpha stands for. A material exported with alpha 0
/// (the lens of the MAN Lion's City rear lamps, 740 faces) is not invisible in OMSI: it only
/// becomes translucent through `[matl_alpha]`, so a zero is read as opaque.
#[must_use]
pub fn effective_diffuse_alpha(alpha: f32) -> f32 {
    if alpha < 0.01 {
        1.0
    } else {
        alpha.clamp(0.0, 1.0)
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
    let a = effective_diffuse_alpha(a);
    let base_color = Color::srgba(
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    );

    let [er, eg, eb] = mat.emissive;
    let emissive = LinearRgba::new(er.max(0.0), eg.max(0.0), eb.max(0.0), 1.0);

    let perceptual_roughness = specular_power_to_roughness(mat.power);

    // Within 0.01 of opaque is opaque, as `effective_diffuse_alpha` reads within 0.01 of zero.
    // A blended mesh writes no depth, so a 0.99 roof let the seats and the framing behind it show.
    let alpha_mode = if a < 0.99 {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };

    StandardMaterial {
        base_color,
        emissive_texture: emission_texture(emissive, texture_handle.as_ref()),
        base_color_texture: texture_handle,
        emissive,
        perceptual_roughness,
        reflectance: specular_to_reflectance(mat.specular),
        metallic: 0.0,
        alpha_mode,
        ..Default::default()
    }
}

/// The texture an OMSI material's self-illumination shows: its own texture. In OMSI's fixed-function
/// lighting the emissive colour joins the light that **multiplies the texture** (a display with
/// emissive 1 shows its picture at full brightness, the Lion's City's VDV page, the MB C2's display
/// line); Bevy adds `emissive` on top without the texture unless it is the emissive texture too,
/// which turned such surfaces into flat white.
#[must_use]
pub fn emission_texture(
    emissive: LinearRgba,
    texture: Option<&Handle<Image>>,
) -> Option<Handle<Image>> {
    (emissive.red > 0.0 || emissive.green > 0.0 || emissive.blue > 0.0)
        .then(|| texture.cloned())
        .flatten()
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
        emissive_texture: emission_texture(emissive, texture_handle.as_ref()),
        base_color_texture: texture_handle,
        emissive,
        perceptual_roughness,
        reflectance: specular_to_reflectance(mat.specular),
        metallic: 0.0,
        alpha_mode,
        ..Default::default()
    }
}

/// Whether a `[matl_alpha]` mode lets the texture's alpha channel take effect. Mode 0 (and no
/// entry at all) is opaque: the alpha channel is unrelated data there. Corpus evidence: the
/// textures of the `Ampelmast_*` traffic-light poles and of many houses are fully transparent
/// but declare `[matl_alpha] 0`, and the objects stand on the maps (Q-SCO-001 in
/// `docs/src/formats/text/sco.md`).
#[must_use]
pub fn uses_texture_alpha(mode: Option<u32>) -> bool {
    matches!(mode, Some(m) if m != 0)
}

pub mod pbr_audit;

#[cfg(test)]
mod tests;
