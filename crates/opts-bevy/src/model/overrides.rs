//! Texture overrides, glass and alpha composition of model textures.

use super::*;

mod alpha;
mod light;
mod transmap;

pub(super) use alpha::compose_alpha;
pub use light::compose_light;
pub(super) use transmap::{composed_opaque_share, transmap_glass};
pub use transmap::{opaque_share_rgba, transmap_glass_rgba, TRANSMAP_OPAQUE, TRANSMAP_SPLIT_SHARE};

/// Texture override for one `.o3d` material slot (from `[matl]` in `.sco` or `model.cfg`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureOverride {
    /// `.o3d` material index the texture applies to.
    pub slot: usize,
    /// Texture file reference.
    pub texture: String,
    /// `[matl_alpha]` mode: `1` = cutout (alpha test), `2` = blend; `None` = opaque.
    pub alpha: Option<u32>,
    /// The script variable that scales the material's alpha (`[alphascale]`). An alpha overlay
    /// (rain, dirt, condensation) is built and follows its variable at run time
    /// (`ModelPart::alpha_variable`); see [`TextureOverride::runtime_alpha_variable`].
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
    /// `[matl_nightmap]`: the night texture, drawn as it is (lit windows, a display's backlight)
    /// when the night switch (`NightlightA`) is on; used when the material has no light map.
    pub night_map: Option<String>,
    /// `[matl_texadress_clamp]`: the texture is sampled at its edges, not repeated.
    pub clamp: bool,
    /// `[matl_freetex]`: the string variable that names the texture to show instead of
    /// `texture` at run time (reported in [`ModelPart::free_texture`]).
    pub free_texture: Option<String>,
}

impl TextureOverride {
    /// Whether the material is an alpha overlay: a `[matl_alpha]` mode other than 0 or a
    /// `[matl_transmap]` texture. `[alphascale]` also appears on opaque base materials
    /// (`Envir_Brightness` on the body of a Crealis bus, on the night-lit skyline blocks of the
    /// Chicago map); alpha-scaling something that does not use alpha is no reason to treat it as an
    /// overlay (P-1014, MI-0029).
    #[must_use]
    pub fn is_alpha_overlay(&self) -> bool {
        crate::material::uses_texture_alpha(self.alpha) || self.trans_map.is_some()
    }

    /// Whether the part is a weather overlay (rain on the windows, dirt, condensation) whose
    /// variable is one of the [`overlay_class`] names.
    #[must_use]
    pub fn is_weather_overlay_part(&self) -> bool {
        self.is_alpha_overlay()
            && self
                .alpha_variable
                .as_deref()
                .is_some_and(is_weather_overlay)
    }

    /// The variable the part follows at run time: set for every alpha overlay with an
    /// `[alphascale]` variable. Weather overlays are built too; their alpha is the product of the
    /// texture alpha and the script variable, and the part is hidden while that product is 0
    /// (`ModelPart::alpha_variable`, `crate::vehicle_lights::vehicle_alpha_scale_system`).
    #[must_use]
    pub fn runtime_alpha_variable(&self) -> Option<&str> {
        self.alpha_variable
            .as_deref()
            .filter(|_| self.is_alpha_overlay())
    }
}

/// The weather classes of overlay variables (M-0120). The stock scripts drive them: `Dirt_*` from
/// the wash and wiper scripts, `Rain_Window_*` from the precipitation, `Beschlag_*` (condensation)
/// from the humidity, `Precip_*` (the precipitation mirror).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayClass {
    /// `Dirt_Norm`, `Dirt_Wiped`, ...
    Dirt,
    /// `Rain_Window_*_Wetness`.
    RainWindow,
    /// `Beschlag_*`.
    Condensation,
    /// `Precip_*`.
    Precipitation,
}

/// The weather class of an overlay variable, `None` for any other variable.
#[must_use]
pub fn overlay_class(variable: &str) -> Option<OverlayClass> {
    let v = variable.trim().to_ascii_lowercase();
    [
        ("rain_window_", OverlayClass::RainWindow),
        ("dirt_", OverlayClass::Dirt),
        ("beschlag_", OverlayClass::Condensation),
        ("precip_", OverlayClass::Precipitation),
    ]
    .into_iter()
    .find(|(prefix, _)| v.starts_with(prefix))
    .map(|(_, class)| class)
}

/// Whether the variable belongs to a weather overlay class ([`overlay_class`]).
#[must_use]
pub fn is_weather_overlay(variable: &str) -> bool {
    overlay_class(variable).is_some()
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
