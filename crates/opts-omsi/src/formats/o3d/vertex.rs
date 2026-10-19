//! Vertex buffer decoder for `.o3d` files.
//!
//! Corresponds to `docs/src/formats/binary/o3d.md#0x17--vertex-chunk` and
//! `docs/src/formats/binary/o3d-chunks.md#0x17-vertices`.

use std::io::{Cursor, Read};

use glam::{Vec2, Vec3};

use crate::formats::o3d::chunks::{expect_tag, TAG_VERTEX};
use crate::formats::o3d::error::{O3dParseError, Result};
use crate::formats::o3d::header::Header;

/// Standard byte stride of a single vertex record in `.o3d` format (32 bytes).
pub const VERTEX_STRIDE: usize = 32;

/// A single decoded 3D vertex.
///
/// In OMSI `.o3d` mesh space:
/// - `position`: +X is right, +Y is up, +Z is forward (left-handed DirectX convention).
/// - `normal`: Surface normal vector (typically unit length).
/// - `uv`: Texture coordinate; (0,0) is top-left, V increases downwards.
/// - `color`: Optional RGBA vertex color (None in standard 32-byte format).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// 3D position vector in mesh coordinates (metres).
    pub position: Vec3,
    /// 3D surface normal vector.
    pub normal: Vec3,
    /// 2D texture coordinates (U, V).
    pub uv: Vec2,
    /// Optional RGBA vertex color components.
    pub color: Option<[f32; 4]>,
}

/// Decodes the vertex buffer chunk (`0x17`).
///
/// # Errors
///
/// Returns [`O3dParseError::Truncated`] if the input stream terminates before all declared vertices are read.
/// Returns [`O3dParseError::UnknownChunk`] if the leading chunk tag is not `0x17`.
pub fn parse_vertex_chunk(reader: &mut Cursor<&[u8]>, header: &Header) -> Result<Vec<Vertex>> {
    expect_tag(reader, TAG_VERTEX)?;

    let count_pos = reader.position() as usize;
    let vertex_count = if header.is_32bit_counts() {
        let mut count_buf = [0u8; 4];
        reader.read_exact(&mut count_buf).map_err(|_| {
            let actual = reader.get_ref().len().saturating_sub(count_pos);
            O3dParseError::Truncated {
                expected: 4,
                actual,
            }
        })?;
        u32::from_le_bytes(count_buf) as usize
    } else {
        let mut count_buf = [0u8; 2];
        reader.read_exact(&mut count_buf).map_err(|_| {
            let actual = reader.get_ref().len().saturating_sub(count_pos);
            O3dParseError::Truncated {
                expected: 2,
                actual,
            }
        })?;
        u16::from_le_bytes(count_buf) as usize
    };

    let start_pos = reader.position() as usize;
    let slice = reader.get_ref();
    let remaining_bytes = slice.len().saturating_sub(start_pos);
    let required_bytes =
        vertex_count
            .checked_mul(VERTEX_STRIDE)
            .ok_or(O3dParseError::Truncated {
                expected: usize::MAX,
                actual: remaining_bytes,
            })?;

    if remaining_bytes < required_bytes {
        return Err(O3dParseError::Truncated {
            expected: required_bytes,
            actual: remaining_bytes,
        });
    }

    let vertex_slice = &slice[start_pos..start_pos + required_bytes];
    let mut vertices = Vec::with_capacity(vertex_count);

    for chunk in vertex_slice.chunks_exact(VERTEX_STRIDE) {
        let pos_x = f32::from_le_bytes(read_f32_le(chunk, 0)?);
        let pos_y = f32::from_le_bytes(read_f32_le(chunk, 4)?);
        let pos_z = f32::from_le_bytes(read_f32_le(chunk, 8)?);

        let n_x = f32::from_le_bytes(read_f32_le(chunk, 12)?);
        let n_y = f32::from_le_bytes(read_f32_le(chunk, 16)?);
        let n_z = f32::from_le_bytes(read_f32_le(chunk, 20)?);

        let u = f32::from_le_bytes(read_f32_le(chunk, 24)?);
        let v = f32::from_le_bytes(read_f32_le(chunk, 28)?);

        vertices.push(Vertex {
            position: Vec3::new(pos_x, pos_y, pos_z),
            normal: Vec3::new(n_x, n_y, n_z),
            uv: Vec2::new(u, v),
            color: None,
        });
    }

    reader.set_position((start_pos + required_bytes) as u64);
    Ok(vertices)
}

fn read_f32_le(slice: &[u8], offset: usize) -> Result<[u8; 4]> {
    let mut bytes = [0u8; 4];
    if let Some(src) = slice.get(offset..offset + 4) {
        bytes.copy_from_slice(src);
        Ok(bytes)
    } else {
        Err(O3dParseError::VertexFormatMismatch {
            expected: VERTEX_STRIDE,
            actual: slice.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vertex_bytes(pos: [f32; 3], norm: [f32; 3], uv: [f32; 2]) -> [u8; 32] {
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(&pos[0].to_le_bytes());
        buf[4..8].copy_from_slice(&pos[1].to_le_bytes());
        buf[8..12].copy_from_slice(&pos[2].to_le_bytes());
        buf[12..16].copy_from_slice(&norm[0].to_le_bytes());
        buf[16..20].copy_from_slice(&norm[1].to_le_bytes());
        buf[20..24].copy_from_slice(&norm[2].to_le_bytes());
        buf[24..28].copy_from_slice(&uv[0].to_le_bytes());
        buf[28..32].copy_from_slice(&uv[1].to_le_bytes());
        buf
    }

    #[test]
    fn test_valid_vertex_buffer_v1() {
        let mut bytes = vec![TAG_VERTEX];
        bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 vertex (v1 uses u16)
        bytes.extend_from_slice(&create_test_vertex_bytes(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [0.5, 0.5],
        ));

        let header = Header {
            version: 1,
            flags: 0,
            stamp: None,
        };
        let mut cursor = Cursor::new(&bytes[..]);
        let verts = parse_vertex_chunk(&mut cursor, &header).expect("valid vertex chunk");
        assert_eq!(verts.len(), 1);
        assert_eq!(verts[0].position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(verts[0].normal, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(verts[0].uv, Vec2::new(0.5, 0.5));
        assert_eq!(verts[0].color, None);
        assert_eq!(cursor.position(), bytes.len() as u64);
    }

    #[test]
    fn test_valid_vertex_buffer_v7() {
        let mut bytes = vec![TAG_VERTEX];
        bytes.extend_from_slice(&2u32.to_le_bytes()); // 2 vertices (v7 uses u32)
        bytes.extend_from_slice(&create_test_vertex_bytes(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
        bytes.extend_from_slice(&create_test_vertex_bytes(
            [5.0, 6.0, 7.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0],
        ));

        let header = Header {
            version: 7,
            flags: 0,
            stamp: Some(0),
        };
        let mut cursor = Cursor::new(&bytes[..]);
        let verts = parse_vertex_chunk(&mut cursor, &header).expect("valid v7 vertices");
        assert_eq!(verts.len(), 2);
        assert_eq!(verts[1].position, Vec3::new(5.0, 6.0, 7.0));
    }

    #[test]
    fn test_vertex_buffer_shorter_than_declared() {
        let mut bytes = vec![TAG_VERTEX];
        bytes.extend_from_slice(&2u32.to_le_bytes()); // declares 2 vertices = 64 bytes
        bytes.extend_from_slice(&create_test_vertex_bytes(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0],
        )); // only provides 1 vertex = 32 bytes

        let header = Header {
            version: 7,
            flags: 0,
            stamp: None,
        };
        let mut cursor = Cursor::new(&bytes[..]);
        let err = parse_vertex_chunk(&mut cursor, &header).unwrap_err();
        assert!(matches!(err, O3dParseError::Truncated { .. }));
    }

    #[test]
    fn test_unaligned_byte_access_safety() {
        // Test that reading float components from unaligned offsets works safely without UB
        let unaligned_raw = [0x55, 0x00, 0x00, 0x80, 0x3F, 0xAA]; // 1.0f32 starting at offset 1
        let slice = &unaligned_raw[1..5];
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(slice);
        let val = f32::from_le_bytes(bytes);
        assert_eq!(val, 1.0f32);
    }
}
