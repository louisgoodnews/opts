//! Material slots and their modifiers (`[matl]` and `[matl_*]`): environment, light, bump and
//! dynamic textures.

/// Environment map reflection parameters for a material slot.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvMap {
    /// Environment cubemap texture file path.
    pub texture: String,
    /// Reflection intensity factor.
    pub factor: f32,
}

/// Script-controlled lightmap for a material slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightMap {
    /// Lightmap texture file path.
    pub texture: String,
    /// Script variable name that modulates light intensity.
    pub variable: String,
}

/// Bump map for a material slot.
#[derive(Debug, Clone, PartialEq)]
pub struct BumpMap {
    /// Normal/bump texture file path.
    pub texture: String,
    /// Bump depth factor.
    pub factor: f32,
}

/// Dynamic texture slot bound to a script variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeTex {
    /// Base texture identifier.
    pub texture: String,
    /// Script variable bound to the dynamic texture.
    pub variable: String,
}

/// Conditional texture swap modifier (`[matl_change]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureChange {
    /// Replacement texture file path.
    pub texture: String,
    /// Sub-index.
    pub index: u32,
    /// Script variable activating the substitution.
    pub variable: String,
}

/// Material slot selection and modifier attributes (`[matl]`, `[matl_*]`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelMaterial {
    /// Texture filename referenced in `[matl]`.
    pub texture: String,
    /// Material slot index in the 3D mesh (0-based).
    pub slot_index: u32,
    /// Alpha blend mode (1 = cutout/test, 2 = blend).
    pub alpha_mode: Option<u32>,
    /// Script variable modulating alpha transparency (`[alphascale]`).
    pub alpha_scale_var: Option<String>,
    /// Script variable modulating vertical UV translation (`[texcoordtransY]`).
    pub tex_coord_trans_y_var: Option<String>,
    /// Environment reflection map.
    pub env_map: Option<EnvMap>,
    /// Environment reflection mask texture path.
    pub env_map_mask: Option<String>,
    /// Lightmap texture and driver variable.
    pub light_map: Option<LightMap>,
    /// Emissive night texture path.
    pub night_map: Option<String>,
    /// Bump map texture and factor.
    pub bump_map: Option<BumpMap>,
    /// Optional transparency map path.
    pub trans_map: Option<String>,
    /// Dynamic text texture index bound via `[useTextTexture]`.
    pub text_texture_index: Option<usize>,
    /// Dynamic script texture index bound via `[useScriptTexture]` or `[matl_transmap] \S:n`.
    pub script_texture_index: Option<usize>,
    /// Dynamic script texture binding via `[matl_freetex]`.
    pub free_tex: Option<FreeTex>,
    /// Alternate textures swapped conditionally.
    pub texture_changes: Vec<TextureChange>,
    /// Disables depth buffer writing (`[matl_noZwrite]`).
    pub no_z_write: bool,
    /// Disables depth buffer testing (`[matl_noZcheck]`).
    pub no_z_check: bool,
    /// Texture clamping flag (`[matl_texadress_clamp]`).
    pub clamp_addressing: bool,
    /// Unhandled material modifiers.
    pub unknown_modifiers: Vec<(String, Vec<String>)>,
}

/// Type alias for [`ModelMaterial`] as requested by milestone specifications.
pub type MaterialModifier = ModelMaterial;
