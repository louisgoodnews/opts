//! Quad and polygon face triangulation for DirectX `.x` meshes.
//!
//! OMSI in-engine mutation tests (T10) proved that 4-vertex quad faces
//! `4; i0, i1, i2, i3` are split into two CCW triangles:
//! - Triangle 1: `[i0, i1, i2]`
//! - Triangle 2: `[i0, i2, i3]`
//!
//! Both triangles share the face's material assignment from `MeshMaterialList`.

use crate::formats::x::error::XParseError;

/// Triangulates a single face polygon into a sequence of CCW triangle index tuples.
///
/// # Arguments
/// * `indices` - Vertex indices comprising the face polygon (length 3 or 4).
/// * `vertex_count` - Total number of available vertices (for bounds checking).
///
/// # Errors
/// * Returns [`XParseError::UnsupportedFaceVertexCount`] if `indices.len() != 3 && indices.len() != 4`.
/// * Returns [`XParseError::IndexOutOfBounds`] if any vertex index is `>= vertex_count`.
pub fn triangulate_face(
    indices: &[u32],
    vertex_count: usize,
) -> Result<Vec<[u32; 3]>, XParseError> {
    for &idx in indices {
        if idx as usize >= vertex_count {
            return Err(XParseError::IndexOutOfBounds {
                index: idx,
                vertex_count,
            });
        }
    }

    match indices.len() {
        3 => Ok(vec![[indices[0], indices[1], indices[2]]]),
        4 => Ok(vec![
            [indices[0], indices[1], indices[2]],
            [indices[0], indices[2], indices[3]],
        ]),
        other => Err(XParseError::UnsupportedFaceVertexCount(other as u32)),
    }
}

/// Converts a set of raw face polygons into triangulated indices and per-triangle material slots.
///
/// When a quad face is split into two triangles, both resulting triangles inherit
/// the original face's material index.
///
/// # Arguments
/// * `raw_faces` - Polygons with variable vertex count (3 or 4).
/// * `face_materials` - Optional per-face material slot indices.
/// * `vertex_count` - Total vertex count for bounds validation.
pub fn triangulate_faces_with_materials(
    raw_faces: &[Vec<u32>],
    face_materials: Option<&[u32]>,
    vertex_count: usize,
) -> Result<(Vec<[u32; 3]>, Vec<u32>), XParseError> {
    let mut triangles = Vec::new();
    let mut materials = Vec::new();

    for (face_idx, face) in raw_faces.iter().enumerate() {
        let tri_list = triangulate_face(face, vertex_count)?;
        let mat_id = face_materials
            .and_then(|m| m.get(face_idx).copied())
            .unwrap_or(0);

        for tri in tri_list {
            triangles.push(tri);
            materials.push(mat_id);
        }
    }

    Ok((triangles, materials))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangulate_triangle() {
        let tris = triangulate_face(&[0, 1, 2], 3).expect("triangle should pass");
        assert_eq!(tris, vec![[0, 1, 2]]);
    }

    #[test]
    fn test_triangulate_quad() {
        let tris = triangulate_face(&[0, 1, 2, 3], 4).expect("quad should pass");
        assert_eq!(tris, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn test_rejects_unsupported_polygon() {
        let err = triangulate_face(&[0, 1, 2, 3, 4], 10).unwrap_err();
        assert_eq!(err, XParseError::UnsupportedFaceVertexCount(5));
    }

    #[test]
    fn test_index_out_of_bounds() {
        let err = triangulate_face(&[0, 5, 2], 3).unwrap_err();
        assert_eq!(
            err,
            XParseError::IndexOutOfBounds {
                index: 5,
                vertex_count: 3,
            }
        );
    }
}
