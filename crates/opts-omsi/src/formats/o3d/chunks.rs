//! Chunk tags, dispatcher, and optional chunk decoders for `.o3d` files.
//!
//! Corresponds to `docs/src/formats/binary/o3d.md#general-structure` and
//! `docs/src/formats/binary/o3d-chunks.md`.

use std::io::{Cursor, Read};

use crate::formats::o3d::error::{O3dParseError, Result};

/// Tag byte for the vertex buffer chunk (`0x17`).
pub const TAG_VERTEX: u8 = 0x17;

/// Tag byte for the face/index buffer chunk (`0x49`).
pub const TAG_FACE: u8 = 0x49;

/// Tag byte for the material table chunk (`0x26`).
pub const TAG_MATERIAL: u8 = 0x26;

/// Tag byte for the optional 4×4 transform matrix chunk (`0x79`).
pub const TAG_MATRIX: u8 = 0x79;

/// Tag byte for the optional bone weights / skinning chunk (`0x54`).
pub const TAG_BONE_WEIGHTS: u8 = 0x54;

/// Represents an unknown or deferred chunk whose payload is preserved as raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownChunk {
    /// Single-byte chunk identifier tag.
    pub tag: u8,
    /// Raw payload bytes of the chunk (excluding the tag byte).
    pub data: Vec<u8>,
}

/// Peeks at the next tag byte without advancing the cursor.
#[must_use]
pub fn peek_tag(reader: &Cursor<&[u8]>) -> Option<u8> {
    let pos = reader.position() as usize;
    reader.get_ref().get(pos).copied()
}

/// Expects and consumes a specific chunk tag byte.
///
/// # Errors
///
/// Returns [`O3dParseError::Truncated`] if at EOF.
/// Returns [`O3dParseError::UnknownChunk`] if the tag byte does not match `expected_tag`.
pub fn expect_tag(reader: &mut Cursor<&[u8]>, expected_tag: u8) -> Result<()> {
    let offset = reader.position() as usize;
    let mut tag_buf = [0u8; 1];
    reader.read_exact(&mut tag_buf).map_err(|_| {
        let actual = reader.get_ref().len().saturating_sub(offset);
        O3dParseError::Truncated {
            expected: 1,
            actual,
        }
    })?;

    if tag_buf[0] != expected_tag {
        return Err(O3dParseError::UnknownChunk(u32::from(tag_buf[0]), offset));
    }

    Ok(())
}

/// Decodes the optional 4×4 transform matrix chunk (`0x79`).
///
/// The matrix consists of 16 little-endian IEEE-754 single-precision floats (64 bytes).
///
/// # Errors
///
/// Returns [`O3dParseError::Truncated`] if fewer than 65 bytes remain.
/// Returns [`O3dParseError::UnknownChunk`] if the leading tag byte is not `0x79`.
pub fn parse_matrix_chunk(reader: &mut Cursor<&[u8]>) -> Result<[[f32; 4]; 4]> {
    expect_tag(reader, TAG_MATRIX)?;

    let mut matrix = [[0.0f32; 4]; 4];
    for row in &mut matrix {
        for cell in row {
            let offset = reader.position() as usize;
            let mut float_buf = [0u8; 4];
            reader.read_exact(&mut float_buf).map_err(|_| {
                let actual = reader.get_ref().len().saturating_sub(offset);
                O3dParseError::Truncated {
                    expected: 4,
                    actual,
                }
            })?;
            *cell = f32::from_le_bytes(float_buf);
        }
    }

    Ok(matrix)
}

/// Decodes the bone weights chunk (`0x54`) into an [`UnknownChunk`] structure.
///
/// Per `docs/src/formats/binary/o3d.md#0x54--bone-weight-chunk-optional`, bone weights
/// are parsed structurally and preserved for deferred evaluation.
///
/// # Errors
///
/// Returns [`O3dParseError::Truncated`] if the chunk stream ends unexpectedly.
/// Returns [`O3dParseError::UnknownChunk`] if the leading tag byte is not `0x54`.
pub fn parse_bone_weights_chunk(reader: &mut Cursor<&[u8]>) -> Result<UnknownChunk> {
    let start_offset = reader.position() as usize;
    expect_tag(reader, TAG_BONE_WEIGHTS)?;

    let payload_start = reader.position() as usize;
    let mut count_buf = [0u8; 2];
    read_chunk_bytes(reader, &mut count_buf, 2)?;
    let bone_count = u16::from_le_bytes(count_buf);

    for _ in 0..bone_count {
        let mut name_len_buf = [0u8; 1];
        read_chunk_bytes(reader, &mut name_len_buf, 1)?;
        let name_len = name_len_buf[0] as usize;

        let mut name_buf = vec![0u8; name_len];
        read_chunk_bytes(reader, &mut name_buf, name_len)?;

        let mut entries_buf = [0u8; 2];
        read_chunk_bytes(reader, &mut entries_buf, 2)?;
        let entry_count = u16::from_le_bytes(entries_buf) as usize;

        // Each entry is 6 bytes: u16 vertex_index + f32 weight
        let entries_total_bytes = entry_count.saturating_mul(6);
        let mut entries_bytes = vec![0u8; entries_total_bytes];
        read_chunk_bytes(reader, &mut entries_bytes, entries_total_bytes)?;
    }

    let payload_end = reader.position() as usize;
    let data = reader.get_ref()[payload_start..payload_end].to_vec();

    tracing::debug!(
        offset = start_offset,
        bone_count = bone_count,
        bytes = data.len(),
        "parsed bone weights chunk 0x54"
    );

    Ok(UnknownChunk {
        tag: TAG_BONE_WEIGHTS,
        data,
    })
}

fn read_chunk_bytes(reader: &mut Cursor<&[u8]>, buf: &mut [u8], expected_len: usize) -> Result<()> {
    let offset = reader.position() as usize;
    reader.read_exact(buf).map_err(|_| {
        let actual = reader.get_ref().len().saturating_sub(offset);
        O3dParseError::Truncated {
            expected: expected_len,
            actual,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_matrix_chunk() {
        let mut bytes = vec![TAG_MATRIX];
        #[rustfmt::skip]
        let matrix_floats: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            3.0, 4.0, 5.0, 1.0,
        ];
        for f in matrix_floats {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        let mut cursor = Cursor::new(&bytes[..]);
        let mat = parse_matrix_chunk(&mut cursor).expect("valid matrix chunk");
        assert_eq!(mat[0], [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(mat[3], [3.0, 4.0, 5.0, 1.0]);
        assert_eq!(cursor.position(), 65);
    }

    #[test]
    fn test_truncated_matrix_chunk() {
        let bytes = [TAG_MATRIX, 1, 2, 3];
        let mut cursor = Cursor::new(&bytes[..]);
        let err = parse_matrix_chunk(&mut cursor).unwrap_err();
        assert!(matches!(err, O3dParseError::Truncated { .. }));
    }

    #[test]
    fn test_matrix_chunk_wrong_tag() {
        let bytes = [0x17, 0, 0, 0];
        let mut cursor = Cursor::new(&bytes[..]);
        let err = parse_matrix_chunk(&mut cursor).unwrap_err();
        assert_eq!(err, O3dParseError::UnknownChunk(0x17, 0));
    }

    #[test]
    fn test_valid_bone_weights_chunk() {
        let mut bytes = vec![TAG_BONE_WEIGHTS];
        // bone_count = 1
        bytes.extend_from_slice(&1u16.to_le_bytes());
        // name_len = 4, "Bone"
        bytes.push(4);
        bytes.extend_from_slice(b"Bone");
        // entry_count = 1
        bytes.extend_from_slice(&1u16.to_le_bytes());
        // vertex_index = 0, weight = 1.0
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&1.0f32.to_le_bytes());

        let mut cursor = Cursor::new(&bytes[..]);
        let chunk = parse_bone_weights_chunk(&mut cursor).expect("valid bone chunk");
        assert_eq!(chunk.tag, TAG_BONE_WEIGHTS);
        assert_eq!(chunk.data.len(), bytes.len() - 1);
    }
}

/// Weights of one bone from the bone-weight chunk (`0x54`).
#[derive(Debug, Clone, PartialEq)]
pub struct BoneInfluence {
    /// Bone name (`Hip`, `Head`, `OS_L`, …).
    pub name: String,
    /// `(vertex index, weight)` entries.
    pub entries: Vec<(u16, f32)>,
}

/// Decodes the payload of a preserved `0x54` chunk (the bytes after the tag).
///
/// Returns `None` if the payload is truncated.
#[must_use]
pub fn decode_bone_weights(data: &[u8]) -> Option<Vec<BoneInfluence>> {
    let mut pos = 0usize;
    let take = |pos: &mut usize, n: usize| -> Option<&[u8]> {
        let s = data.get(*pos..*pos + n)?;
        *pos += n;
        Some(s)
    };
    let count = u16::from_le_bytes(take(&mut pos, 2)?.try_into().ok()?);
    let mut bones = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let name_len = take(&mut pos, 1)?[0] as usize;
        let name = String::from_utf8_lossy(take(&mut pos, name_len)?).into_owned();
        let n = u16::from_le_bytes(take(&mut pos, 2)?.try_into().ok()?) as usize;
        let mut entries = Vec::with_capacity(n);
        for _ in 0..n {
            let e = take(&mut pos, 6)?;
            entries.push((
                u16::from_le_bytes([e[0], e[1]]),
                f32::from_le_bytes([e[2], e[3], e[4], e[5]]),
            ));
        }
        bones.push(BoneInfluence { name, entries });
    }
    Some(bones)
}
