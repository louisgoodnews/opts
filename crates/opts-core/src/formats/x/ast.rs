//! In-memory data models representing parsed DirectX `.x` meshes and frames.
//!
//! Provides structured representations for vertices, triangulated face indices,
//! materials, and hierarchical frame nodes.

use glam::{Vec2, Vec3};

/// A single vertex containing 3D position, normal, and UV coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XVertex {
    /// 3D position in model space (X, Y, Z).
    pub position: Vec3,
    /// Surface normal vector.
    pub normal: Vec3,
    /// 2D texture coordinates (U, V).
    pub uv: Vec2,
}

impl XVertex {
    /// Creates a new `XVertex`.
    #[inline]
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }
}

/// Surface material properties parsed from a DirectX `Material` template.
#[derive(Debug, Clone, PartialEq)]
pub struct XMaterial {
    /// Optional name of the material.
    pub name: Option<String>,
    /// Diffuse surface color (RGBA in 0.0..=1.0).
    pub diffuse: [f32; 4],
    /// Specular power / glossiness exponent.
    pub power: f32,
    /// Specular color (RGB in 0.0..=1.0).
    pub specular: [f32; 3],
    /// Emissive self-illumination color (RGB in 0.0..=1.0).
    pub emissive: [f32; 3],
    /// Optional relative path of the texture file.
    pub texture_filename: Option<String>,
}

impl Default for XMaterial {
    fn default() -> Self {
        Self {
            name: None,
            diffuse: [1.0, 1.0, 1.0, 1.0],
            power: 0.0,
            specular: [0.0, 0.0, 0.0],
            emissive: [0.0, 0.0, 0.0],
            texture_filename: None,
        }
    }
}

/// A parsed sub-mesh containing geometry, triangulated indices, and material slots.
#[derive(Debug, Clone, PartialEq)]
pub struct XSubMesh {
    /// Optional identifier name of this mesh (e.g. `Quad`).
    pub name: Option<String>,
    /// Vertex buffer.
    pub vertices: Vec<XVertex>,
    /// Triangulated face vertex indices (each entry is 3 vertex indices).
    pub indices: Vec<[u32; 3]>,
    /// Material slot ID for each triangle in `indices`.
    pub face_materials: Vec<u32>,
    /// Material palette referenced by `face_materials`.
    pub materials: Vec<XMaterial>,
}

/// A hierarchical frame node with an optional transform matrix and child meshes.
#[derive(Debug, Clone, PartialEq)]
pub struct XFrame {
    /// Optional identifier name of this frame node (e.g. `Root`).
    pub name: Option<String>,
    /// Optional 4×4 affine transformation matrix stored as row vectors.
    pub transform: Option<[[f32; 4]; 4]>,
    /// Meshes attached directly to this frame.
    pub meshes: Vec<XSubMesh>,
    /// Nested child frame nodes.
    pub children: Vec<XFrame>,
}

/// Complete in-memory representation of a parsed DirectX `.x` asset.
#[derive(Debug, Clone, PartialEq)]
pub struct XMesh {
    /// Hierarchical root frames defined in the file.
    pub frames: Vec<XFrame>,
    /// Any meshes defined at top-level outside of an explicit frame.
    pub top_level_meshes: Vec<XSubMesh>,
    /// Global materials defined at top-level.
    pub global_materials: Vec<XMaterial>,
}

impl XMesh {
    /// Returns the primary mesh if this asset contains only one mesh.
    pub fn primary_mesh(&self) -> Option<&XSubMesh> {
        if let Some(m) = self.top_level_meshes.first() {
            return Some(m);
        }
        for frame in &self.frames {
            if let Some(m) = frame.meshes.first() {
                return Some(m);
            }
        }
        None
    }

    /// Collects references to all sub-meshes throughout frames and top-level definitions.
    pub fn all_meshes(&self) -> Vec<&XSubMesh> {
        let mut result = Vec::new();
        result.extend(self.top_level_meshes.iter());
        for frame in &self.frames {
            collect_frame_meshes(frame, &mut result);
        }
        result
    }
}

fn collect_frame_meshes<'a>(frame: &'a XFrame, out: &mut Vec<&'a XSubMesh>) {
    out.extend(frame.meshes.iter());
    for child in &frame.children {
        collect_frame_meshes(child, out);
    }
}
