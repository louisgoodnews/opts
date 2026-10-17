//! Abstract Syntax Tree (AST) data models for vehicle model configurations (`model.cfg`).

use glam::Vec3;
use std::collections::BTreeMap;

/// Viewpoint visibility bitmask controlling whether a mesh is visible in different camera views.
///
/// Bit values defined by OMSI:
/// - `0`: Always visible in all views.
/// - `1`: Visible only from outside the vehicle.
/// - `2`: Visible only from inside the passenger compartment/cab.
/// - `4`: Visible only when vehicle is driven as an AI bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewpointFlags(pub u32);

impl ViewpointFlags {
    /// Always visible in every camera viewpoint.
    pub const ALWAYS: Self = Self(0);
    /// Visible from outside viewpoints.
    pub const OUTSIDE: Self = Self(1);
    /// Visible from interior cab and passenger viewpoints.
    pub const INSIDE: Self = Self(2);
    /// Visible when the vehicle is an AI-controlled bus.
    pub const AI_BUS: Self = Self(4);

    /// Creates a new viewpoint flags mask.
    #[must_use]
    pub const fn new(mask: u32) -> Self {
        Self(mask)
    }

    /// Returns the raw bitmask value.
    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Checks if the mesh is configured to be unconditionally visible.
    #[must_use]
    pub const fn is_always(&self) -> bool {
        self.0 == 0
    }

    /// Checks if the mesh is visible from external camera views.
    #[must_use]
    pub const fn is_outside(&self) -> bool {
        self.is_always() || (self.0 & 1) != 0
    }

    /// Checks if the mesh is visible from interior passenger and driver views.
    #[must_use]
    pub const fn is_inside(&self) -> bool {
        self.is_always() || (self.0 & 2) != 0
    }

    /// Checks if the mesh is visible when the vehicle is controlled by AI.
    #[must_use]
    pub const fn is_ai(&self) -> bool {
        self.is_always() || (self.0 & 4) != 0
    }
}

/// Conditional mesh visibility rule based on a script variable.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshVisibility {
    /// Script variable name to evaluate.
    pub variable: String,
    /// Value expected for this mesh to remain visible.
    pub expected_value: f32,
}

/// Skeletal bone attachment for skinned articulation meshes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshBone {
    /// Bone identifier string.
    pub name: String,
    /// Numerical bone slot. Human skeleton slots are negative (`-2` … `-14`).
    pub index: i32,
}

/// Variable-driven animation action (translation or rotation).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimAction {
    /// Script variable name that drives this animation transform.
    pub variable: String,
    /// Scaling multiplier applied to the variable value.
    pub factor: f32,
}

/// Category of an animation action.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimType {
    /// Rotation transform around the origin axis.
    Rotation(AnimAction),
    /// Translation transform along the origin axis.
    Translation(AnimAction),
}

/// Animation definition for moving and rotating vehicle meshes (`[newanim]`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelAnim {
    /// Translation offset of the animation pivot origin in local mesh space.
    pub origin_trans: Option<Vec3>,
    /// Rotation angles of the animation pivot origin in degrees (X, Y, Z).
    pub origin_rot: Vec3,
    /// Indicates whether the animation origin is derived directly from the mesh pivot.
    pub origin_from_mesh: bool,
    /// Variable-driven rotation action.
    pub anim_rot: Option<AnimAction>,
    /// Variable-driven translation action.
    pub anim_trans: Option<AnimAction>,
    /// Damping or delay parameter for motion smoothing.
    pub delay: Option<f32>,
    /// Maximum animation speed limit.
    pub maxspeed: Option<f32>,
    /// Animation start offset.
    pub offset: Option<f32>,
    /// General animation driving variable name (`anim_val`).
    pub anim_val: Option<String>,
    /// Maximum value bound for the driving variable (`anim_maxval`).
    pub anim_maxval: Option<f32>,
    /// Unhandled sub-properties preserved for round-trip fidelity.
    pub unknown_properties: Vec<(String, Vec<String>)>,
}

impl ModelAnim {
    /// Returns true if this animation includes a rotation action.
    #[must_use]
    pub fn is_rotation(&self) -> bool {
        self.anim_rot.is_some()
    }

    /// Returns true if this animation includes a translation action.
    #[must_use]
    pub fn is_translation(&self) -> bool {
        self.anim_trans.is_some()
    }

    /// Returns the primary driving variable name if present.
    #[must_use]
    pub fn driver_variable(&self) -> Option<&str> {
        if let Some(ref rot) = self.anim_rot {
            Some(&rot.variable)
        } else if let Some(ref trans) = self.anim_trans {
            Some(&trans.variable)
        } else {
            self.anim_val.as_deref()
        }
    }
}

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

/// Interior light source (`[interiorlight]`).
#[derive(Debug, Clone, PartialEq)]
pub struct InteriorLight {
    /// Script variable controlling illumination state.
    pub variable: String,
    /// Effective illumination range in meters.
    pub range: f32,
    /// Color in RGB (0-255).
    pub color_rgb: [u8; 3],
    /// 3D position vector in vehicle space.
    pub pos: Vec3,
}

/// Focused exterior spotlight / headlight (`[spotlight]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Spotlight {
    /// Position of the spotlight emitter in local space.
    pub pos: Vec3,
    /// Direction vector of the beam.
    pub dir: Vec3,
    /// RGB color (0-255).
    pub color_rgb: [u8; 3],
    /// Maximum projection range in meters.
    pub range: f32,
    /// Inner cone angle in degrees.
    pub inner_cone: f32,
    /// Outer cone angle in degrees.
    pub outer_cone: f32,
}

/// A 3D mesh element and all scoped properties attached to it (`[mesh]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelMesh {
    /// Relative path to the `.o3d` or `.x` geometry file.
    pub path: String,
    /// Viewpoint visibility mask.
    pub viewpoint: ViewpointFlags,
    /// Four interior light index assignments (`[illumination_interior]`).
    pub illumination_interior: Option<[i32; 4]>,
    /// Conditional visibility rules.
    pub visibility: Vec<MeshVisibility>,
    /// Click event trigger name (`[mouseevent]`).
    pub mouseevent: Option<String>,
    /// Mesh identifier for parent-child animation linking (`[mesh_ident]`).
    pub mesh_ident: Option<String>,
    /// Identifier of the parent mesh to inherit transforms from (`[animparent]`).
    pub animparent: Option<String>,
    /// Flag indicating whether smooth skinning is enabled (`[smoothskin]`).
    pub smoothskin: bool,
    /// Skeletal bone bindings (`[setbone]`).
    pub bones: Vec<MeshBone>,
    /// Flag indicating whether this mesh casts shadows (`[isshadow]`).
    pub isshadow: bool,
    /// Animations attached to this mesh (`[newanim]`).
    pub anims: Vec<ModelAnim>,
    /// Material slot modifiers (`[matl]`).
    pub materials: Vec<ModelMaterial>,
    /// Interior lights attached to this mesh.
    pub interior_lights: Vec<InteriorLight>,
    /// Spotlights attached to this mesh.
    pub spotlights: Vec<Spotlight>,
    /// Unrecognized keywords scoped to this mesh.
    pub unknown_fields: BTreeMap<String, Vec<Vec<String>>>,
}

impl ModelMesh {
    /// Creates a new mesh with default properties.
    #[must_use]
    pub fn new(path: String) -> Self {
        Self {
            path,
            viewpoint: ViewpointFlags::ALWAYS,
            illumination_interior: None,
            visibility: Vec::new(),
            mouseevent: None,
            mesh_ident: None,
            animparent: None,
            smoothskin: false,
            bones: Vec::new(),
            isshadow: false,
            anims: Vec::new(),
            materials: Vec::new(),
            interior_lights: Vec::new(),
            spotlights: Vec::new(),
            unknown_fields: BTreeMap::new(),
        }
    }
}

/// Level of Detail (LOD) tier partitioning vehicle geometry (`[LOD]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelLod {
    /// Distance threshold for this LOD tier (e.g. 0.0 or 0.1).
    pub threshold: f32,
    /// Meshes rendered at this LOD level.
    pub meshes: Vec<ModelMesh>,
}

/// Dynamic text texture rendered from a script variable (`[texttexture]`, `[texttexture_enh]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextTexture {
    /// Script string variable name supplying text.
    pub variable: String,
    /// Bitmap font file or font ident.
    pub font_name: String,
    /// Raster texture width in pixels.
    pub width: u32,
    /// Raster texture height in pixels.
    pub height: u32,
    /// Monochrome (`false`) or full color (`true`).
    pub full_color: bool,
    /// User color in RGB (0-255).
    pub color_rgb: [u8; 3],
    /// Enhanced alignment: 0 = centered, 1 = left, 2 = right.
    pub orientation: Option<u32>,
    /// Pixel grid alignment.
    pub grid: Option<u32>,
}

/// Script-generated texture (`[scripttexture]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptTexture {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
}

/// Color texture change scheme (`[CTC]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtcScheme {
    /// Name of the color scheme.
    pub name: String,
    /// Texture directory path.
    pub texture_path: String,
    /// Default index.
    pub default_index: i32,
}

/// Color texture change texture declaration (`[CTCTexture]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtcTexture {
    /// Texture ident.
    pub name: String,
    /// Texture file path.
    pub texture_path: String,
}

/// Root data model representing a vehicle rendering manifest (`model.cfg`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelConfig {
    /// Global texture detail scaling factor from `[tex_detail_factor]`.
    pub tex_detail_factor: Option<f32>,
    /// Color texture change schemes from `[CTC]`.
    pub ctc_schemes: Vec<CtcScheme>,
    /// Color texture change texture mappings from `[CTCTexture]`.
    pub ctc_textures: Vec<CtcTexture>,
    /// Dynamic text texture declarations.
    pub text_textures: Vec<TextTexture>,
    /// Dynamic script texture declarations.
    pub script_textures: Vec<ScriptTexture>,
    /// Level of detail tiers and mesh hierarchies.
    pub lods: Vec<ModelLod>,
    /// Unscoped interior lights.
    pub interior_lights: Vec<InteriorLight>,
    /// Unscoped spotlights.
    pub spotlights: Vec<Spotlight>,
    /// Light effects (`[light_enh]`, `[light_enh_2]`) in declaration order, whatever mesh or LOD
    /// group they follow (recorded in `after_mesh`/`lod_threshold`, the mesh index counted within
    /// its LOD group). Same layout as in a `.sco` ([`crate::formats::scenery::ScoLight`]).
    pub lights: Vec<crate::formats::scenery::ScoLight>,
    /// Blocks that were not understood (malformed light effects), with the reason.
    pub warnings: Vec<String>,
    /// Global unrecognized keywords.
    pub unknown_fields: BTreeMap<String, Vec<Vec<String>>>,
}

impl ModelConfig {
    /// Creates an empty model configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The meshes drawn at close range: those before the first `[LOD]` (always visible) together
    /// with the first explicit `[LOD]` group (the full-detail level). Later groups are simplified
    /// stand-ins for distance. A file whose first group is already an explicit `[LOD]` yields that
    /// group alone.
    ///
    /// The implicit group before the first marker is recognised by its threshold of 0 followed by
    /// an explicit group with a positive threshold (`[LOD] 0.07` ... `[LOD] 0` in the Aachen
    /// Citaro files).
    #[must_use]
    pub fn detail_lod(&self) -> Option<ModelLod> {
        let first = self.lods.first()?;
        match self.lods.get(1) {
            Some(second) if first.threshold <= 0.0 && second.threshold > 0.0 => {
                let mut merged = second.clone();
                merged.meshes.splice(0..0, first.meshes.iter().cloned());
                Some(merged)
            }
            _ => Some(first.clone()),
        }
    }

    /// Returns the total count of meshes across all LOD tiers.
    #[must_use]
    pub fn total_mesh_count(&self) -> usize {
        self.lods.iter().map(|lod| lod.meshes.len()).sum()
    }
}
