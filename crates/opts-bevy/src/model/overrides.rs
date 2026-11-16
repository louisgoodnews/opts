//! Texture overrides, glass and alpha composition of model textures.

use super::*;

mod alpha;
mod light;
mod transmap;

pub use light::compose_light;
pub(super) use alpha::compose_alpha;
pub use transmap::{transmap_glass_rgba, TRANSMAP_OPAQUE, TRANSMAP_SPLIT_SHARE};
pub(super) use transmap::transmap_glass;

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
    /// `[matl_nightmap]`: the night texture, drawn as it is (lit windows, a display's backlight)
    /// when the night switch (`NightlightA`) is on; used when the material has no light map.
    pub night_map: Option<String>,
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
