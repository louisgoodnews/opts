//! Single entry point parser for `.o3d` files.
//!
//! Corresponds to the high-level reader algorithm described in
//! `docs/src/formats/binary/o3d-chunks.md#reader-algorithm-descriptive`.

use std::io::Cursor;

use crate::formats::o3d::chunks::{
    parse_bone_weights_chunk, parse_matrix_chunk, peek_tag, TAG_BONE_WEIGHTS, TAG_MATRIX,
};
use crate::formats::o3d::error::{O3dParseError, Result};
use crate::formats::o3d::header::Header;
use crate::formats::o3d::index::parse_face_chunk;
use crate::formats::o3d::material::parse_material_chunk;
use crate::formats::o3d::vertex::parse_vertex_chunk;
use crate::formats::o3d::O3d;

/// Parses a byte buffer into a normalized [`O3d`] mesh structure.
///
/// This is the primary public entry point for parsing `.o3d` files.
///
/// # Errors
///
/// Returns [`O3dParseError`] if the byte slice is truncated, contains an invalid magic sequence,
/// an unsupported version, out-of-bounds indices, or unknown chunk tags.
pub fn parse(bytes: &[u8]) -> Result<O3d> {
    let mut cursor = Cursor::new(bytes);

    let header = Header::parse(&mut cursor)?;
    let vertices = parse_vertex_chunk(&mut cursor, &header)?;
    let faces = parse_face_chunk(&mut cursor, &header, vertices.len())?;

    let mut indices = Vec::with_capacity(faces.len());
    let mut face_materials = Vec::with_capacity(faces.len());
    for face in faces {
        indices.push(face.indices);
        face_materials.push(face.material_index);
    }

    let materials = parse_material_chunk(&mut cursor)?;

    let mut transform = None;
    let mut unknown_chunks = Vec::new();

    while (cursor.position() as usize) < bytes.len() {
        match peek_tag(&cursor) {
            Some(TAG_MATRIX) => {
                let matrix = parse_matrix_chunk(&mut cursor)?;
                transform = Some(matrix);
            }
            Some(TAG_BONE_WEIGHTS) => {
                let bone_chunk = parse_bone_weights_chunk(&mut cursor)?;
                unknown_chunks.push(bone_chunk);
            }
            Some(tag) => {
                let offset = cursor.position() as usize;
                return Err(O3dParseError::UnknownChunk(u32::from(tag), offset));
            }
            None => break,
        }
    }

    Ok(O3d {
        version: header.version,
        header_flags: u32::from(header.flags),
        vertices,
        indices,
        face_materials,
        materials,
        transform,
        unknown_chunks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constructs the self-authored v1 sample documented in `o3d.md#-examples`.
    fn build_self_authored_v1_sample() -> Vec<u8> {
        let mut bytes = Vec::new();

        // 1. Header (3 bytes): 84 19 01
        bytes.extend_from_slice(&[0x84, 0x19, 0x01]);

        // 2. Vertex chunk (0x17): tag + u16 count (3) + 3 * 32 bytes
        bytes.push(0x17);
        bytes.extend_from_slice(&3u16.to_le_bytes());
        for i in 0..3 {
            let pos = [i as f32, 0.0f32, 0.0f32];
            let norm = [0.0f32, 1.0f32, 0.0f32];
            let uv = [0.0f32, 0.0f32];
            for f in pos {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            for f in norm {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            for f in uv {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
        }

        // 3. Face chunk (0x49): tag + u16 count (1) + (indices 0, 2, 1, mat 0)
        bytes.push(0x49);
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // mat 0

        // 4. Material chunk (0x26): tag + u16 count (1) + 44 bytes floats + string
        bytes.push(0x26);
        bytes.extend_from_slice(&1u16.to_le_bytes());
        let diff = [0.8f32, 0.8f32, 0.8f32, 1.0f32];
        let spec = [0.0f32, 0.0f32, 0.0f32];
        let emis = [0.0f32, 0.0f32, 0.0f32];
        let power = 96.0f32;
        for f in diff {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        for f in spec {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        for f in emis {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        bytes.extend_from_slice(&power.to_le_bytes());
        bytes.push(8); // name_len = 8
        bytes.extend_from_slice(b"test.bmp");

        // 5. Matrix chunk (0x79): tag + 16 floats (identity)
        bytes.push(0x79);
        #[rustfmt::skip]
        let matrix: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        for f in matrix {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        bytes
    }

    #[test]
    fn test_parse_self_authored_v1_sample() {
        let sample = build_self_authored_v1_sample();
        let o3d = parse(&sample).expect("self-authored v1 sample should parse");

        assert_eq!(o3d.version, 1);
        assert_eq!(o3d.header_flags, 0);
        assert_eq!(o3d.vertices.len(), 3);
        assert_eq!(o3d.indices.len(), 1);
        assert_eq!(o3d.indices[0], [0, 2, 1]);
        assert_eq!(o3d.face_materials, vec![0]);
        assert_eq!(o3d.materials.len(), 1);
        assert_eq!(o3d.materials[0].primary_texture(), Some("test.bmp"));
        assert!(o3d.transform.is_some());
        assert!(o3d.unknown_chunks.is_empty());
    }

    #[test]
    fn test_parse_unexpected_trailing_tag() {
        let mut sample = build_self_authored_v1_sample();
        sample.push(0x99); // trailing garbage tag

        let err = parse(&sample).unwrap_err();
        assert!(matches!(err, O3dParseError::UnknownChunk(0x99, _)));
    }
}
