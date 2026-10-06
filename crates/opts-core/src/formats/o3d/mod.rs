//! OMSI `.o3d` 3D mesh format parser.
//!
//! Corresponds to `docs/src/formats/binary/o3d.md` and
//! `docs/src/formats/binary/o3d-chunks.md`.

pub mod chunks;
pub mod error;
pub mod header;
pub mod index;
pub mod material;
pub mod parse;
pub mod vertex;

pub use chunks::{
    parse_bone_weights_chunk, parse_matrix_chunk, UnknownChunk, TAG_BONE_WEIGHTS, TAG_FACE,
    TAG_MATERIAL, TAG_MATRIX, TAG_VERTEX,
};
pub use error::{O3dParseError, Result};
pub use header::Header;
pub use index::{parse_face_chunk, Face, FACE_STRIDE_16BIT, FACE_STRIDE_32BIT};
pub use material::{parse_material_chunk, Material, MATERIAL_FIXED_BYTES};
pub use parse::parse;
pub use vertex::{parse_vertex_chunk, Vertex, VERTEX_STRIDE};

/// Normalized in-memory representation of an OMSI `.o3d` 3D mesh file.
#[derive(Debug, Clone, PartialEq)]
pub struct O3d {
    /// File format version (1, 3, 4, 5, or 7).
    pub version: u32,
    /// Header flags value (0 for v1; bit 0 indicates 32-bit face indices).
    pub header_flags: u32,
    /// Decoded vertex buffer.
    pub vertices: Vec<Vertex>,
    /// Triangle face index buffer (`[i0, i1, i2]`).
    pub indices: Vec<[u32; 3]>,
    /// Per-face material table indices corresponding to each triangle in `indices`.
    pub face_materials: Vec<u16>,
    /// Material parameters and texture definitions.
    pub materials: Vec<Material>,
    /// Optional 4×4 transform matrix from chunk `0x79`.
    pub transform: Option<[[f32; 4]; 4]>,
    /// Unparsed raw chunks preserved for future analysis (e.g. bone weights `0x54`).
    pub unknown_chunks: Vec<UnknownChunk>,
}

impl O3d {
    /// Returns the total number of vertices in the mesh.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the total number of triangles in the mesh.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    /// Returns the total number of materials in the material table.
    #[must_use]
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }
}
