//! OMSI `.o3d` 3D mesh format parser.
//!
//! Corresponds to `docs/src/formats/binary/o3d.md` and
//! `docs/src/formats/binary/o3d-chunks.md`.

pub mod audit;
pub mod chunks;
pub mod error;
pub mod exact;
pub mod header;
pub mod index;
pub mod material;
pub mod parse;
pub mod vertex;

pub use chunks::{
    decode_bone_weights, parse_bone_weights_chunk, parse_matrix_chunk, BoneInfluence, UnknownChunk,
    TAG_BONE_WEIGHTS, TAG_FACE, TAG_MATERIAL, TAG_MATRIX, TAG_VERTEX,
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
    /// Per-file key of the header (versions >= 4) when it is not the `0xFFFF_FFFF` "plain"
    /// marker. Files with a key were written by an exporter that scrambles vertex data with a
    /// private, key-seeded scheme (see `docs/src/formats/binary/o3d.md`); plain files never are.
    pub encryption_key: Option<u32>,
    /// Set when [`exact::decode`] reproduced the generator of a keyed mesh: positions, texture
    /// coordinates and normals are the originals, and the mesh is read like a plain file.
    pub exact_decoded: bool,
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

    /// Bone weights from chunk `0x54`, if the file has them (skinned meshes such as humans).
    #[must_use]
    pub fn bone_influences(&self) -> Option<Vec<BoneInfluence>> {
        self.unknown_chunks
            .iter()
            .find(|c| c.tag == 0x54)
            .and_then(|c| decode_bone_weights(&c.data))
    }

    /// Fraction of triangles whose geometric normal agrees (|cos| > 0.5, either orientation)
    /// with the average of their vertex normals, or `None` if fewer than 8 triangles are testable.
    ///
    /// Well-formed meshes score near 1.0. Meshes whose vertex positions and normals disagree in
    /// axis labelling (observed in many v5 files, consistent with the author-protected /
    /// encrypted meshes the OMSI Blender add-on mentions) score near chance.
    #[must_use]
    pub fn normal_coherence(&self) -> Option<f32> {
        let (mut ok, mut total) = (0u32, 0u32);
        for t in &self.indices {
            let (Some(a), Some(b), Some(c)) = (
                self.vertices.get(t[0] as usize),
                self.vertices.get(t[1] as usize),
                self.vertices.get(t[2] as usize),
            ) else {
                continue;
            };
            let face = (b.position - a.position).cross(c.position - a.position);
            let vn = a.normal + b.normal + c.normal;
            let (fl, vl) = (face.length(), vn.length());
            if fl < 1e-6 || vl < 1e-3 {
                continue;
            }
            total += 1;
            if (face.dot(vn) / (fl * vl)).abs() > 0.5 {
                ok += 1;
            }
        }
        (total >= 8).then(|| ok as f32 / total as f32)
    }

    /// Fraction of consecutive triangle pairs that share an edge and form a parallelogram
    /// (`a + c = b + d`), or `None` with fewer than 6 such pairs.
    ///
    /// Rectangular quads are the bulk of building meshes, so coherent vertex positions score
    /// high while positions whose components are permuted per vertex score near zero. The
    /// measure is independent of the stored normals.
    #[must_use]
    pub fn position_coherence(&self) -> Option<f32> {
        let (mut pairs, mut quads) = (0u32, 0u32);
        for w in self.indices.windows(2) {
            let (a, c) = (w[0], w[1]);
            let shared: Vec<u32> = a.iter().copied().filter(|x| c.contains(x)).collect();
            if shared.len() != 2 || shared[0] == shared[1] {
                continue;
            }
            let (Some(&oa), Some(&oc)) = (
                a.iter().find(|x| !shared.contains(x)),
                c.iter().find(|x| !shared.contains(x)),
            ) else {
                continue;
            };
            let pos = |i: u32| self.vertices.get(i as usize).map(|v| v.position);
            let (Some(pa), Some(pc), Some(ps0), Some(ps1)) =
                (pos(oa), pos(oc), pos(shared[0]), pos(shared[1]))
            else {
                continue;
            };
            pairs += 1;
            if (pa + pc - ps0 - ps1).length() < 1e-3 {
                quads += 1;
            }
        }
        (pairs >= 6).then(|| quads as f32 / pairs as f32)
    }

    /// Whether two axes of the vertex bounding box have the *identical* interval
    /// (extent > 8 m, not centred on zero). That happens when some vertices carry a coordinate in
    /// the wrong slot (a tower's height appearing in both the `x` and the `y` slot), which the
    /// parallelogram measure misses because most faces are still intact.
    #[must_use]
    pub fn has_leaked_axis(&self) -> bool {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for t in &self.indices {
            for &i in t {
                if let Some(v) = self.vertices.get(i as usize) {
                    for (k, c) in [v.position.x, v.position.y, v.position.z]
                        .into_iter()
                        .enumerate()
                    {
                        lo[k] = lo[k].min(c);
                        hi[k] = hi[k].max(c);
                    }
                }
            }
        }
        (0..3).any(|a| {
            (a + 1..3).any(|b| {
                let ext = hi[a] - lo[a];
                ext > 8.0
                    && (lo[a] - lo[b]).abs() < 0.01
                    && (hi[a] - hi[b]).abs() < 0.01
                    && (lo[a] + hi[a]).abs() > 0.1 * ext
            })
        })
    }

    /// Axes (0 = x, 1 = y, 2 = z) whose minimum lies on zero, i.e. the axes the object stands on.
    /// Objects are modelled on the ground, so exactly one such axis identifies "up".
    #[must_use]
    pub fn floor_axes(&self) -> Vec<usize> {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for t in &self.indices {
            for &i in t {
                if let Some(v) = self.vertices.get(i as usize) {
                    for (k, c) in [v.position.x, v.position.y, v.position.z]
                        .into_iter()
                        .enumerate()
                    {
                        lo[k] = lo[k].min(c);
                        hi[k] = hi[k].max(c);
                    }
                }
            }
        }
        (0..3)
            .filter(|&k| lo[k].abs() < 0.02 && hi[k] - lo[k] > 0.3)
            .collect()
    }

    /// Classifies how the vertex data of this mesh must be interpreted.
    #[must_use]
    pub fn vertex_layout(&self) -> VertexLayout {
        // Every keyed file is scrambled: measured on 5,673 keyed meshes with a plain twin, 91 %
        // of those the coherence test calls `Standard` and 82 % of the `AxisCycled` ones differ
        // from the twin in more than 20 % of their vertices (normals are scrambled too, so the
        // normal-agreement test cannot see it). Only a twin can make them readable.
        if self.encryption_key.is_some() && !self.exact_decoded {
            return VertexLayout::Unreadable;
        }
        let layout = self.classify_by_coherence();
        // Plain files are never scrambled: whatever looks incoherent there is merely a mesh with
        // unusable normals (poles, signs, hedges), so positions can be trusted.
        if layout == VertexLayout::Unreadable && self.encryption_key.is_none() {
            return VertexLayout::RecomputedNormals;
        }
        layout
    }

    fn classify_by_coherence(&self) -> VertexLayout {
        match self.normal_coherence() {
            None => VertexLayout::Standard,
            Some(n) if n >= 0.8 => VertexLayout::Standard,
            Some(_) => match self.position_coherence() {
                Some(p) if p >= 0.5 && !self.has_leaked_axis() => {
                    match self.floor_axes().as_slice() {
                        [1] => VertexLayout::RecomputedNormals,
                        [0] => VertexLayout::AxisCycled,
                        // No unique floor axis (e.g. a building with a basement below zero): fall
                        // back on the version, since every axis-cycled file observed is v4/v5
                        // while Y-up files with bad normals are v1.
                        _ if self.version >= 4 => VertexLayout::AxisCycled,
                        _ => VertexLayout::RecomputedNormals,
                    }
                }
                _ => VertexLayout::Unreadable,
            },
        }
    }
}

/// How the vertex data of an `.o3d` has to be interpreted (see [`O3d::vertex_layout`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexLayout {
    /// Positions and normals agree: `x` lateral, `y` up, `z` forward.
    Standard,
    /// Positions are intact and Y-up (the object stands on `y = 0`) but the stored normals are
    /// unusable (e.g. hedges). Normals must be recomputed from the faces.
    RecomputedNormals,
    /// Positions are coherent but the stored normals do not match them. Observed in many v5
    /// building meshes, whose geometry matches `(lateral, up, forward) = (-y, x, z)` of the file:
    /// `x` is height (floor at 0), `z` the long axis along the object's heading. Normals must be
    /// recomputed from the faces.
    AxisCycled,
    /// Positions themselves are permuted per vertex; the mesh cannot be reconstructed.
    Unreadable,
}
