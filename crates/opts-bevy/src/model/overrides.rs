//! Texture overrides, glass and alpha composition of model textures.

use super::*;

/// Texture override for one `.o3d` material slot (from `[matl]` in `.sco` or `model.cfg`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureOverride {
    /// `.o3d` material index the texture applies to.
    pub slot: usize,
    /// Texture file reference.
    pub texture: String,
    /// `[matl_alpha]` mode: `1` = cutout (alpha test), `2` = blend; `None` = opaque.
    pub alpha: Option<u32>,
    /// The script variable that scales the material's alpha (`[alphascale]`). An overlay that uses
    /// alpha and whose variable OPTS never raises (rain, dirt) is not built at all, see
    /// [`TextureOverride::is_variable_hidden`]; any other alpha-scaled material is built and
    /// follows its variable at run time (`ModelPart::alpha_variable`).
    pub alpha_variable: Option<String>,
    /// `[matl_transmap]` texture: its values become the alpha channel of the material's texture.
    pub trans_map: Option<String>,
    /// Ready-made image (rendered text) used instead of loading `texture`.
    pub image: Option<Handle<Image>>,
    /// `[matl_lightmap]` texture and the script variable that switches it. The light map lights the
    /// material's own texture: the emissive texture is texture × light map (see
    /// [`compose_light`]), with an emissive strength of 0; the variable is reported in
    /// [`ModelPart::light_variable`] for the runtime to drive (`crate::vehicle_lights`).
    pub light_map: Option<LightMap>,
    /// `[matl_freetex]`: the string variable that names the texture to show instead of
    /// `texture` at run time (reported in [`ModelPart::free_texture`]).
    pub free_texture: Option<String>,
}

impl TextureOverride {
    /// Whether the slot is an overlay that stays invisible because OPTS never raises its variable.
    ///
    /// `[alphascale]` also appears on opaque base materials (`Envir_Brightness` on the body of a
    /// Crealis bus, on the night-lit skyline blocks of the Chicago map): alpha-scaling something
    /// that does not use alpha is no reason to drop the surface, the whole body of such a vehicle
    /// vanished (P-1014, MI-0029). Only materials with a `[matl_alpha]` mode other than 0 or a
    /// `[matl_transmap]` texture are overlays.
    ///
    /// Of those, only the overlays of weather the simulation does not have ([`is_static_overlay`]:
    /// rain on the windows, dirt) are dropped at load. The others — glass scaled by
    /// `Envir_Brightness` (6,651 materials), `szyby`, `para_*` — belong on the vehicle and follow
    /// their variable (MAN SD83 without window glass, 2026-10-14).
    #[must_use]
    pub fn is_variable_hidden(&self) -> bool {
        self.alpha_variable
            .as_deref()
            .is_some_and(is_static_overlay)
            && (crate::material::uses_texture_alpha(self.alpha) || self.trans_map.is_some())
    }

    /// The variable the part follows at run time: set for an alpha-using material scaled by a
    /// variable that is not a static overlay.
    #[must_use]
    pub fn runtime_alpha_variable(&self) -> Option<&str> {
        self.alpha_variable
            .as_deref()
            .filter(|v| !is_static_overlay(v))
            .filter(|_| crate::material::uses_texture_alpha(self.alpha) || self.trans_map.is_some())
    }
}

/// Variables of weather the simulation does not have: their overlays (rain on windows, dirt) are
/// always 0 and not worth an entity (`Rain_Window_*_Wetness` 22,888 uses, `Dirt_*` 15,733 in the
/// installed models). When weather or dirt is simulated they stop being static.
#[must_use]
pub fn is_static_overlay(variable: &str) -> bool {
    let v = variable.trim().to_ascii_lowercase();
    ["rain_window_", "dirt_", "precip_"]
        .iter()
        .any(|p| v.starts_with(p))
}

pub(super) fn base_name(path: &str) -> String {
    path.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

/// Finds the `[matl]` entry that applies to material `slot` of a mesh.
///
/// `[matl] <texture> <n>` names the **native texture** of the material it modifies and `n`
/// selects the n-th material (0-based) that uses that texture. This is how one mesh can carry
/// several overlay slots with the same texture (dirt, rain) and how a glass slot gets its own
/// alpha settings (measured on 275,721 entries: 65 % are consistent with this reading, 38 % with
/// "`n` is the slot index"). When the named texture matches no native slot, the entry is a
/// texture *replacement* and `n` is the slot index.
pub fn find_override<'a>(
    materials: &[o3d::Material],
    slot: usize,
    overrides: &'a [TextureOverride],
) -> Option<&'a TextureOverride> {
    let native: Vec<String> = materials
        .iter()
        .map(|m| base_name(m.primary_texture().unwrap_or("")))
        .collect();
    let name = native.get(slot)?;
    let occurrence = native[..slot].iter().filter(|n| *n == name).count();
    overrides
        .iter()
        .find(|o| o.slot == occurrence && base_name(&o.texture) == *name)
        .or_else(|| {
            overrides
                .iter()
                .find(|o| o.slot == slot && !native.contains(&base_name(&o.texture)))
        })
}

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
pub(super) fn transmap_glass(composed: &Image) -> Option<Image> {
    let data = composed.data.as_deref()?;
    let (w, h) = (composed.width() as usize, composed.height() as usize);
    let glass = transmap_glass_rgba(data.get(..w * h * 4)?)?;
    let mut out = composed.clone();
    out.texture_descriptor.mip_level_count = 1;
    out.data = Some(glass);
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}

/// The emissive picture of a light-mapped material: its texture multiplied by the light map, in
/// linear light. OMSI's light maps are masks of the light's colour over the lit areas (the MAN
/// GN92 panel's is one warm patch over all gauges, the MB C2's display a white rectangle): used
/// as the emitted colour itself they paint those areas over; multiplied they light the dials and
/// the display's picture. The light map is sampled at the texture's resolution.
pub fn compose_light(base: &Image, light: &Image) -> Option<Image> {
    let base = crate::texture::decode_to_rgba8(base)?;
    let light = crate::texture::decode_to_rgba8(light)?;
    let (bw, bh) = (base.width() as usize, base.height() as usize);
    let (lw, lh) = (light.width() as usize, light.height() as usize);
    let base_data = base.data.as_deref()?;
    let light_data = light.data.as_deref()?;
    if bw == 0
        || bh == 0
        || lw == 0
        || lh == 0
        || base_data.len() < bw * bh * 4
        || light_data.len() < lw * lh * 4
    {
        return None;
    }
    let to_linear: Vec<f32> = (0..256)
        .map(|v| {
            let c = v as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        })
        .collect();
    let to_srgb = |l: f32| -> u8 {
        let c = if l <= 0.003_130_8 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        };
        (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8
    };
    let mut out = base.clone();
    out.texture_descriptor.mip_level_count = 1;
    let data = out.data.as_mut()?;
    data.truncate(bw * bh * 4);
    for y in 0..bh {
        let ly = y * lh / bh;
        for x in 0..bw {
            let lx = x * lw / bw;
            let o = (y * bw + x) * 4;
            let l = (ly * lw + lx) * 4;
            for c in 0..3 {
                let lin =
                    to_linear[usize::from(data[o + c])] * to_linear[usize::from(light_data[l + c])];
                data[o + c] = to_srgb(lin);
            }
            data[o + 3] = 255;
        }
    }
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}

pub(super) fn compose_alpha(base: &Image, trans: &Image) -> Option<Image> {
    // DDS textures stay block-compressed on the GPU; the texel values are needed here.
    let base = crate::texture::decode_to_rgba8(base)?;
    let trans_decoded = crate::texture::decode_to_rgba8(trans);
    let trans = trans_decoded.as_ref().unwrap_or(trans);
    let base = &base;
    use bevy::render::render_resource::TextureFormat as F;
    let (bw, bh) = (base.width() as usize, base.height() as usize);
    let (tw, th) = (trans.width() as usize, trans.height() as usize);
    let base_data = base.data.as_deref()?;
    let trans_data = trans.data.as_deref()?;
    if !matches!(
        base.texture_descriptor.format,
        F::Rgba8UnormSrgb | F::Rgba8Unorm
    ) || base_data.len() < bw * bh * 4
        || tw == 0
        || th == 0
    {
        return None;
    }
    let sample: Box<dyn Fn(usize) -> u8> = match trans.texture_descriptor.format {
        F::R8Unorm if trans_data.len() >= tw * th => Box::new(|i| trans_data[i]),
        F::Rgba8UnormSrgb | F::Rgba8Unorm if trans_data.len() >= tw * th * 4 => {
            // Use the alpha channel if the map has one, else its luminance.
            let has_alpha = trans_data[..tw * th * 4]
                .chunks_exact(4)
                .any(|p| p[3] != 255);
            Box::new(move |i| {
                let p = &trans_data[i * 4..i * 4 + 4];
                if has_alpha {
                    p[3]
                } else {
                    p[0]
                }
            })
        }
        _ => return None,
    };
    let mut out = base.clone();
    out.texture_descriptor.mip_level_count = 1;
    let out_data = out.data.as_mut()?;
    out_data.truncate(bw * bh * 4);
    for y in 0..bh {
        for x in 0..bw {
            let (tx, ty) = (x * tw / bw, y * th / bh);
            out_data[(y * bw + x) * 4 + 3] = sample(ty * tw + tx);
        }
    }
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}
