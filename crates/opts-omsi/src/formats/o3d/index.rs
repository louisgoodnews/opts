//! Index buffer decoder for `.o3d` files.
//!
//! Corresponds to `docs/src/formats/binary/o3d.md#0x49--face-chunk` and
//! `docs/src/formats/binary/o3d-chunks.md#0x49-faces`.

use std::io::{Cursor, Read};

use crate::formats::o3d::chunks::{expect_tag, TAG_FACE};
use crate::formats::o3d::error::{O3dParseError, Result};
use crate::formats::o3d::header::Header;

/// Byte stride for a 16-bit index face record (3 * 2 + 2 = 8 bytes).
pub const FACE_STRIDE_16BIT: usize = 8;

/// Byte stride for a 32-bit index face record (3 * 4 + 2 = 14 bytes).
pub const FACE_STRIDE_32BIT: usize = 14;

/// A single decoded triangular face with its vertex indices and material assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Face {
    /// Three zero-based vertex indices defining the triangle in CCW front-facing winding.
    pub indices: [u32; 3],
    /// Zero-based index into the material table.
    pub material_index: u16,
}

fn read_face_count(reader: &mut Cursor<&[u8]>, header: &Header) -> Result<usize> {
    let count_pos = reader.position() as usize;
    if header.is_32bit_counts() {
        let mut count_buf = [0u8; 4];
        reader.read_exact(&mut count_buf).map_err(|_| {
            let actual = reader.get_ref().len().saturating_sub(count_pos);
            O3dParseError::Truncated {
                expected: 4,
                actual,
            }
        })?;
        Ok(u32::from_le_bytes(count_buf) as usize)
    } else {
        let mut count_buf = [0u8; 2];
        reader.read_exact(&mut count_buf).map_err(|_| {
            let actual = reader.get_ref().len().saturating_sub(count_pos);
            O3dParseError::Truncated {
                expected: 2,
                actual,
            }
        })?;
        Ok(u16::from_le_bytes(count_buf) as usize)
    }
}

fn parse_faces_32bit(
    face_slice: &[u8],
    face_count: usize,
    vertex_count: usize,
) -> Result<Vec<Face>> {
    let mut faces = Vec::with_capacity(face_count);
    for chunk in face_slice.chunks_exact(FACE_STRIDE_32BIT) {
        let i0 = u32::from_le_bytes(read_u32_bytes(chunk, 0)?);
        let i1 = u32::from_le_bytes(read_u32_bytes(chunk, 4)?);
        let i2 = u32::from_le_bytes(read_u32_bytes(chunk, 8)?);
        let mat_idx = u16::from_le_bytes(read_u16_bytes(chunk, 12)?);

        validate_index(i0, vertex_count)?;
        validate_index(i1, vertex_count)?;
        validate_index(i2, vertex_count)?;

        faces.push(Face {
            indices: [i0, i1, i2],
            material_index: mat_idx,
        });
    }
    Ok(faces)
}

fn parse_faces_16bit(
    face_slice: &[u8],
    face_count: usize,
    vertex_count: usize,
) -> Result<Vec<Face>> {
    let mut faces = Vec::with_capacity(face_count);
    for chunk in face_slice.chunks_exact(FACE_STRIDE_16BIT) {
        let i0 = u32::from(u16::from_le_bytes(read_u16_bytes(chunk, 0)?));
        let i1 = u32::from(u16::from_le_bytes(read_u16_bytes(chunk, 2)?));
        let i2 = u32::from(u16::from_le_bytes(read_u16_bytes(chunk, 4)?));
        let mat_idx = u16::from_le_bytes(read_u16_bytes(chunk, 6)?);

        validate_index(i0, vertex_count)?;
        validate_index(i1, vertex_count)?;
        validate_index(i2, vertex_count)?;

        faces.push(Face {
            indices: [i0, i1, i2],
            material_index: mat_idx,
        });
    }
    Ok(faces)
}

/// Decodes the face/index chunk (`0x49`) into a vector of [`Face`] records.
///
/// Supports both 16-bit and 32-bit index buffers based on `header.is_32bit_indices()`.
/// Validates that all vertex indices are strictly less than `vertex_count`.
///
/// # Errors
///
/// Returns [`O3dParseError::Truncated`] if the input stream terminates before all declared faces are read.
/// Returns [`O3dParseError::UnknownChunk`] if the leading chunk tag is not `0x49`.
/// Returns [`O3dParseError::IndexOutOfBounds`] if any vertex index is `>= vertex_count`.
pub fn parse_face_chunk(
    reader: &mut Cursor<&[u8]>,
    header: &Header,
    vertex_count: usize,
) -> Result<Vec<Face>> {
    expect_tag(reader, TAG_FACE)?;
    let face_count = read_face_count(reader, header)?;
    let is_32bit = header.is_32bit_indices();
    let stride = if is_32bit {
        FACE_STRIDE_32BIT
    } else {
        FACE_STRIDE_16BIT
    };

    let start_pos = reader.position() as usize;
    let slice = reader.get_ref();
    let remaining = slice.len().saturating_sub(start_pos);
    let required = face_count
        .checked_mul(stride)
        .ok_or(O3dParseError::Truncated {
            expected: usize::MAX,
            actual: remaining,
        })?;

    if remaining < required {
        return Err(O3dParseError::Truncated {
            expected: required,
            actual: remaining,
        });
    }

    let face_slice = &slice[start_pos..start_pos + required];
    let faces = if is_32bit {
        parse_faces_32bit(face_slice, face_count, vertex_count)?
    } else {
        parse_faces_16bit(face_slice, face_count, vertex_count)?
    };

    reader.set_position((start_pos + required) as u64);
    Ok(faces)
}

fn validate_index(index: u32, vertex_count: usize) -> Result<()> {
    if (index as usize) >= vertex_count {
        Err(O3dParseError::IndexOutOfBounds {
            index,
            count: vertex_count,
        })
    } else {
        Ok(())
    }
}

fn read_u32_bytes(slice: &[u8], offset: usize) -> Result<[u8; 4]> {
    let mut bytes = [0u8; 4];
    if let Some(src) = slice.get(offset..offset + 4) {
        bytes.copy_from_slice(src);
        Ok(bytes)
    } else {
        Err(O3dParseError::Truncated {
            expected: 4,
            actual: slice.len().saturating_sub(offset),
        })
    }
}

fn read_u16_bytes(slice: &[u8], offset: usize) -> Result<[u8; 2]> {
    let mut bytes = [0u8; 2];
    if let Some(src) = slice.get(offset..offset + 2) {
        bytes.copy_from_slice(src);
        Ok(bytes)
    } else {
        Err(O3dParseError::Truncated {
            expected: 2,
            actual: slice.len().saturating_sub(offset),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_16bit_indices() {
        let mut bytes = vec![TAG_FACE];
        bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 face (v1 count)
        bytes.extend_from_slice(&0u16.to_le_bytes()); // idx 0
        bytes.extend_from_slice(&2u16.to_le_bytes()); // idx 2
        bytes.extend_from_slice(&1u16.to_le_bytes()); // idx 1
        bytes.extend_from_slice(&5u16.to_le_bytes()); // mat idx 5

        let header = Header {
            version: 1,
            flags: 0,
            stamp: None,
        };
        let mut cursor = Cursor::new(&bytes[..]);
        let faces = parse_face_chunk(&mut cursor, &header, 3).expect("valid face chunk");
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].indices, [0, 2, 1]);
        assert_eq!(faces[0].material_index, 5);
        assert_eq!(cursor.position(), bytes.len() as u64);
    }

    #[test]
    fn test_valid_32bit_indices() {
        let mut bytes = vec![TAG_FACE];
        bytes.extend_from_slice(&1u32.to_le_bytes()); // 1 face (v7 count)
        bytes.extend_from_slice(&100u32.to_le_bytes()); // idx 100
        bytes.extend_from_slice(&200u32.to_le_bytes()); // idx 200
        bytes.extend_from_slice(&300u32.to_le_bytes()); // idx 300
        bytes.extend_from_slice(&0u16.to_le_bytes()); // mat idx 0

        let header = Header {
            version: 7,
            flags: 1, // bit 0 set: 32-bit indices
            stamp: Some(0),
        };
        let mut cursor = Cursor::new(&bytes[..]);
        let faces = parse_face_chunk(&mut cursor, &header, 500).expect("valid 32-bit faces");
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].indices, [100, 200, 300]);
        assert_eq!(faces[0].material_index, 0);
    }

    #[test]
    fn test_index_out_of_bounds() {
        let mut bytes = vec![TAG_FACE];
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes()); // idx 3 out of bounds for count = 3
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());

        let header = Header {
            version: 1,
            flags: 0,
            stamp: None,
        };
        let mut cursor = Cursor::new(&bytes[..]);
        let err = parse_face_chunk(&mut cursor, &header, 3).unwrap_err();
        assert_eq!(err, O3dParseError::IndexOutOfBounds { index: 3, count: 3 });
    }

    #[test]
    fn test_truncated_face_buffer() {
        let mut bytes = vec![TAG_FACE];
        bytes.extend_from_slice(&2u16.to_le_bytes()); // declares 2 faces = 16 bytes
        bytes.extend_from_slice(&[0u8; 8]); // only 1 face provided

        let header = Header {
            version: 1,
            flags: 0,
            stamp: None,
        };
        let mut cursor = Cursor::new(&bytes[..]);
        let err = parse_face_chunk(&mut cursor, &header, 10).unwrap_err();
        assert!(matches!(err, O3dParseError::Truncated { .. }));
    }
}
