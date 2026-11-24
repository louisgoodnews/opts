//! The materials and meshes of a scenery object.

use super::*;

/// Material properties attached to a mesh in a scenery object.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoMaterial {
    /// Texture bitmap filename.
    pub texture: String,
    /// Material slot index in the 3D mesh.
    pub material_index: usize,
    /// Alpha transparency mode (0 = opaque, 1 = alpha test, 2 = alpha blend).
    pub alpha_mode: Option<u32>,
    /// Environment reflection map filename and reflection strength factor (0.0..1.0).
    pub env_map: Option<(String, f32)>,
    /// Whether the environment map is masked by alpha.
    pub env_map_mask: bool,
    /// Lightmap bitmap filename.
    pub light_map: Option<String>,
    /// Variable that switches the light map (second line of `[matl_lightmap]`); `None` when the
    /// map names none (then it follows the night lighting).
    pub light_map_variable: Option<String>,
    /// Night texture filename.
    pub night_map: Option<String>,
    /// Whether material transparency scales with `[alphascale]`.
    pub trans_map: bool,
    /// Disable writing to depth buffer (Z-buffer).
    pub no_z_write: bool,
    /// Disable depth testing.
    pub no_z_check: bool,
    /// Free runtime texture binding variable name.
    pub free_tex: Option<String>,
    /// Dynamic material texture change: `(texture, material_index, variable_name)`.
    pub matl_change: Option<(String, usize, String)>,
    /// Variable name driving material alpha scale.
    pub alpha_scale: Option<String>,
    /// Text texture (index into `text_textures`) drawn on this material (`[useTextTexture]`
    /// directly after the `[matl]`).
    pub text_texture: Option<usize>,
}

impl ScoMaterial {
    /// Creates a new material with the specified texture and slot index.
    #[must_use]
    pub fn new(texture: impl Into<String>, material_index: usize) -> Self {
        Self {
            texture: texture.into(),
            material_index,
            alpha_mode: None,
            env_map: None,
            env_map_mask: false,
            light_map: None,
            light_map_variable: None,
            night_map: None,
            trans_map: false,
            no_z_write: false,
            no_z_check: false,
            free_tex: None,
            matl_change: None,
            alpha_scale: None,
            text_texture: None,
        }
    }
}

/// 3D mesh attachment in a scenery object (`[mesh]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoMesh {
    /// Relative path to `.o3d` or `.x` mesh file.
    pub file: String,
    /// Level-of-detail threshold under which this mesh is active.
    pub lod_threshold: Option<f32>,
    /// Whether this specific mesh casts shadows.
    pub cast_shadow: Option<bool>,
    /// Materials overriding or extending this mesh.
    pub materials: Vec<ScoMaterial>,
    /// Conditional visibility: `(variable_name, expected_value)`.
    pub visible: Option<(String, f32)>,
    /// Interaction mouse click event trigger name.
    pub mouse_event: Option<String>,
    /// Text texture slots bound to this mesh (`[useTextTexture]`).
    pub text_texture_indices: Vec<usize>,
    /// Animation definitions attached to this mesh (`[newanim]`, same layout as in `model.cfg`).
    pub animations: Vec<ModelAnim>,
    /// `[onlyeditor]` after this `[mesh]`: the mesh is an editor helper and is not drawn in the
    /// game (e.g. `Trees_MC`'s `treehelper.x` next to the `[tree]` billboard, the second
    /// `Eis.o3d` of `Eisplatte.sco`). The rest of the object is unaffected.
    pub only_editor: bool,
}

impl ScoMesh {
    /// Creates a new mesh attachment with the specified file path.
    #[must_use]
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            lod_threshold: None,
            cast_shadow: None,
            materials: Vec::new(),
            visible: None,
            mouse_event: None,
            text_texture_indices: Vec::new(),
            animations: Vec::new(),
            only_editor: false,
        }
    }
}
