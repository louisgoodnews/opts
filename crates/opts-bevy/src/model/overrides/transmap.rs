//! Transmap glass: the body and glass passes of a texture whose alpha comes from a transmap.

use super::*;

/// Returns `base` with its alpha channel replaced by the values of `trans` (nearest-neighbour
/// resampled). Bases and maps may be RGBA8 or block-compressed (BC1-3); an R8 map is read directly.
/// Only level 0 of either input is read; the result gets a fresh mip chain like any loaded texture.
/// Alpha from which a texel of a transmap-composed texture counts as opaque: the body pass is
/// `AlphaMode::Mask(TRANSMAP_OPAQUE)`, the glass pass draws the rest blended.
pub const TRANSMAP_OPAQUE: f32 = 0.95;

/// Share of opaque texels from which a composed texture is split into body and glass passes.
/// Below it (shadow quads, smoke, plain windows) the material stays one blended pass.
pub const TRANSMAP_SPLIT_SHARE: f32 = 0.5;

/// The glass texels of an RGBA8 buffer whose alpha came from a transmap: `None` when fewer than
/// [`TRANSMAP_SPLIT_SHARE`] of the texels are opaque (≥ [`TRANSMAP_OPAQUE`]), else the buffer with
/// the alpha of every opaque texel set to 0 (the body pass draws those).
#[must_use]
pub fn transmap_glass_rgba(rgba: &[u8]) -> Option<Vec<u8>> {
    // `a / 255 >= 0.95` exactly as the mask test compares it.
    let opaque = |a: u8| f32::from(a) / 255.0 >= TRANSMAP_OPAQUE;
    let texels = rgba.len() / 4;
    if texels == 0 {
        return None;
    }
    let solid = rgba.chunks_exact(4).filter(|p| opaque(p[3])).count();
    if (solid as f32) < TRANSMAP_SPLIT_SHARE * texels as f32 {
        return None;
    }
    let mut glass = rgba.to_vec();
    for p in glass.chunks_exact_mut(4) {
        if opaque(p[3]) {
            p[3] = 0;
        }
    }
    Some(glass)
}

/// [`transmap_glass_rgba`] on a composed image (RGBA8, as [`compose_alpha`] returns it).
///
/// The composed image already carries its mip chain; the glass is built from its base level
/// and gets its own chain, so data and descriptor agree when the image is uploaded.
pub(in crate::model) fn transmap_glass(composed: &Image) -> Option<Image> {
    let data = composed.data.as_deref()?;
    let (w, h) = (composed.width() as usize, composed.height() as usize);
    let glass = transmap_glass_rgba(data.get(..w * h * 4)?)?;
    let mut out = composed.clone();
    out.texture_descriptor.mip_level_count = 1;
    out.data = Some(glass);
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}
