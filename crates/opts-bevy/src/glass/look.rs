//! The glass look applied to a material and restored from its original values.

use super::*;

/// The glass look applied to `m`, from the values it had before. `texture` is the premultiplied
/// copy of the colour texture, when there is one.
pub(super) fn apply_look(m: &mut StandardMaterial, original: &GlassOriginal, texture: Option<Handle<Image>>) {
    m.perceptual_roughness = original.roughness.min(GLASS_ROUGHNESS);
    m.reflectance = original.reflectance.max(GLASS_REFLECTANCE);
    m.alpha_mode = AlphaMode::Premultiplied;
    let c = original.base_color.to_linear();
    // The material's own alpha multiplies the texture alpha: it belongs in the colour as well.
    let (tint, fade) = match (&original.texture, &texture) {
        (Some(_), None) => (GLASS_FALLBACK_TINT, 1.0),
        _ => (GLASS_TINT, c.alpha),
    };
    let k = tint * fade * GLASS_CLARITY;
    m.base_color = Color::linear_rgba(c.red * k, c.green * k, c.blue * k, c.alpha * GLASS_CLARITY);
    if let Some(t) = texture {
        m.base_color_texture = Some(t);
    }
}

pub(super) fn restore(m: &mut StandardMaterial, original: &GlassOriginal) {
    m.perceptual_roughness = original.roughness;
    m.reflectance = original.reflectance;
    m.alpha_mode = original.alpha;
    m.base_color = original.base_color;
    m.base_color_texture = original.texture.clone();
}
