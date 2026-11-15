//! Plain data produced by the model loader: the renderable [`ModelPart`] and the
//! [`ContentStats`] counters shown in the diagnostic overlay.

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::math::Vec3;
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;

/// One renderable submesh of a model: geometry bound to a single material.
#[derive(Debug, Clone)]
pub struct ModelPart {
    /// Mesh in Bevy (right-handed, Y-up) space.
    pub mesh: Handle<Mesh>,
    /// Material with the resolved texture.
    pub material: Handle<StandardMaterial>,
    /// Number of vertices in the mesh.
    pub vertices: usize,
    /// Script variable that scales the material's light map (`[matl_lightmap]`); `None` when the
    /// material has no light map or the map names no variable.
    pub light_variable: Option<String>,
    /// Script variable that scales the material's alpha (`[alphascale]`), for parts that follow it
    /// at run time (window glass); `None` for ordinary parts.
    pub alpha_variable: Option<String>,
    /// The ready-made image that replaced the material's texture (rendered text), if any.
    pub image: Option<Handle<Image>>,
    /// `[matl_freetex]`: the string variable naming the texture shown at run time, and the
    /// material's light map, which lights whatever texture it shows (see `crate::free_texture`).
    pub free_texture: Option<(String, Option<Handle<Image>>)>,
    /// Translation of the mesh file's own `0x79` matrix (the modelling tool's pivot) in Bevy
    /// axes; `None` for files without one and for `.x` meshes (whose frames are applied).
    /// `[newanim] origin_from_mesh` rotates about it.
    pub pivot: Option<Vec3>,
}

/// Counters collected while loading, shown in the diagnostic overlay and logs.
#[derive(Debug, Clone, Default)]
pub struct ContentStats {
    /// Distinct `.o3d` files parsed.
    pub o3d_files: usize,
    /// Total vertices across all parsed `.o3d` files (counted once per file).
    pub o3d_vertices: usize,
    /// Distinct textures decoded.
    pub textures: usize,
    /// Meshes skipped because their geometry is unusable (a keyed file the generator does not
    /// cover, or an implausible extent).
    pub protected_meshes: usize,
    /// Keyed meshes decoded exactly by following the exporter's generator (`o3d::exact`).
    pub exact_meshes: usize,
    /// Meshes read with the axis-cycled layout (coherent positions, unusable stored normals).
    pub axis_cycled_meshes: usize,
    /// Material slots left out because a script variable scales their alpha (`[alphascale]`):
    /// an object whose slots are all of that kind has nothing to draw, which is not a load failure.
    pub hidden_parts: usize,
    /// Transmap materials split into an opaque body and a blended glass part.
    pub transmap_splits: usize,
    /// Human-readable descriptions of assets that failed to load.
    pub failures: Vec<String>,
}
