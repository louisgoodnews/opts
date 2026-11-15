//! Meshes of a vehicle model: the mesh visibility and bone rules, the mesh itself and its LOD tier.

use super::*;
use std::collections::BTreeMap;

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
