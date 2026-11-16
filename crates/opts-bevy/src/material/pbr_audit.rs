//! The audited OMSI-to-PBR mapping of M-0122 (`docs/src/architecture/graphics-overhaul.md`
//! section 5.2, "Mapping function").
//!
//! Pure functions only: nothing here is wired into the material conversion yet. The conversion
//! keeps the legacy mapping ([`super::specular_power_to_roughness`], [`super::specular_to_reflectance`])
//! until the `graphics.material_model` setting exists and the A/B of the reference views is run.
//! The audited roughness weights the power-derived roughness by the brightness of the specular
//! colour: a Direct3D material whose specular colour is black is matte whatever its power.

#[cfg(test)]
mod tests;

/// Rec. 709 luma of an OMSI specular RGB colour, unclamped.
#[must_use]
pub fn specular_luma(specular: [f32; 3]) -> f32 {
    0.2126 * specular[0] + 0.7152 * specular[1] + 0.0722 * specular[2]
}

/// Hermite smoothstep between `edge0` and `edge1`, clamped to `0..=1`.
#[must_use]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Specular luma at which the power-derived roughness is used in full.
pub const FULL_SPECULAR_LUMA: f32 = 0.3;

/// The audited perceptual roughness: the power-derived roughness, faded towards `1.0` (matte) as
/// the specular luma falls to zero, reached in full at [`FULL_SPECULAR_LUMA`].
#[must_use]
pub fn audited_roughness(power: f32, spec_luma: f32) -> f32 {
    let from_power = super::specular_power_to_roughness(power);
    let weight = smoothstep(0.0, FULL_SPECULAR_LUMA, spec_luma);
    1.0 + (from_power - 1.0) * weight
}

/// The audited reflectance of an opaque dielectric: the specular luma, capped at Bevy's `0.5`.
#[must_use]
pub fn audited_reflectance(spec_luma: f32) -> f32 {
    (spec_luma / 0.5).clamp(0.0, 1.0) * 0.5
}
