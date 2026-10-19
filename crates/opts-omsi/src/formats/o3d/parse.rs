//! Single entry point parser for `.o3d` files.
//!
//! Corresponds to the high-level reader algorithm described in
//! `docs/src/formats/binary/o3d-chunks.md#reader-algorithm-descriptive`.

use std::io::Cursor;

use crate::formats::o3d::chunks::{
    parse_bone_weights_chunk, parse_matrix_chunk, peek_tag, TAG_BONE_WEIGHTS, TAG_MATRIX,
};
use crate::formats::o3d::error::Result;
use crate::formats::o3d::header::Header;
use crate::formats::o3d::index::{parse_face_chunk, FACE_STRIDE_16BIT};
use crate::formats::o3d::material::parse_material_chunk;
use crate::formats::o3d::vertex::{parse_vertex_chunk, VERTEX_STRIDE};
use crate::formats::o3d::O3d;

/// Parses a byte buffer into a normalized [`O3d`] mesh structure.
///
/// This is the primary public entry point for parsing `.o3d` files.
///
/// # Errors
///
/// Returns [`super::O3dParseError`] if the byte slice is truncated, contains an invalid magic sequence,
/// an unsupported version, out-of-bounds indices, or unknown chunk tags.
pub fn parse(bytes: &[u8]) -> Result<O3d> {
    match parse_exact(bytes) {
        Ok(o) => Ok(o),
        Err(first) => salvage(bytes)
            .or_else(|| wrapped_v1_counts(bytes))
            .ok_or(first),
    }
}

/// How many leading bytes may precede the magic in a damaged file.
const MAX_LEADING_JUNK: usize = 64;

/// Last-resort repairs for files that real-world mod packs ship damaged. Each repair is only
/// accepted if the result parses completely into a non-empty mesh.
///
/// 1. Leading junk before the `84 19` magic (some exporters prepend a byte or two).
/// 2. Text-mode transfer damage: every `0x00` byte became `0x20`. Detected by a large share of
///    `0x20` bytes in a file that fails normally; reverting them restores the counts and floats
///    (a genuine `0x20` byte inside a float is rare and only perturbs that value).
fn salvage(bytes: &[u8]) -> Option<O3d> {
    let accept = |o: O3d| (!o.vertices.is_empty() && !o.indices.is_empty()).then_some(o);
    if let Some(start) = bytes
        .windows(2)
        .take(MAX_LEADING_JUNK)
        .position(|w| w == [0x84, 0x19])
        .filter(|&i| i > 0)
    {
        if let Some(o) = parse_exact(&bytes[start..]).ok().and_then(accept) {
            return Some(o);
        }
    }
    let spaces = bytes.iter().filter(|&&b| b == 0x20).count();
    if spaces * 10 > bytes.len() {
        let repaired: Vec<u8> = bytes
            .iter()
            .map(|&b| if b == 0x20 { 0x00 } else { b })
            .collect();
        if let Some(o) = parse_exact(&repaired).ok().and_then(accept) {
            return Some(o);
        }
    }
    None
}

/// Version 1 stores the vertex and face counts as `u16`; an exporter that wrote a larger mesh
/// stored the count modulo 65,536 (observed: a station with 69,927 vertices declared as 4,391).
/// The real counts are found by probing `declared + k * 65,536` (vertices and faces) until the stream continues with the
/// face chunk tag after the vertices and the material chunk tag after the faces; the file is then
/// re-framed with 32-bit counts (version 3 layout) and parsed normally.
fn wrapped_v1_counts(bytes: &[u8]) -> Option<O3d> {
    const MAX_WRAPS: usize = 8;
    if bytes.len() < 9 || bytes[..3] != [0x84, 0x19, 0x01] || bytes[3] != 0x17 {
        return None;
    }
    let declared_v = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
    for kv in 0..=MAX_WRAPS {
        let nv = declared_v + kv * 65_536;
        let faces_at = 6 + nv * VERTEX_STRIDE;
        if bytes.get(faces_at) != Some(&0x49) || bytes.len() < faces_at + 3 {
            continue;
        }
        let declared_f = usize::from(u16::from_le_bytes([
            bytes[faces_at + 1],
            bytes[faces_at + 2],
        ]));
        for kf in 0..=MAX_WRAPS {
            if kv == 0 && kf == 0 {
                continue;
            }
            let nf = declared_f + kf * 65_536;
            let mat_at = faces_at + 3 + nf * FACE_STRIDE_16BIT;
            if bytes.get(mat_at) != Some(&0x26) {
                continue;
            }
            let mut out = vec![0x84, 0x19, 0x03, 0x00, 0x17];
            out.extend_from_slice(&(nv as u32).to_le_bytes());
            out.extend_from_slice(&bytes[6..faces_at]);
            out.push(0x49);
            out.extend_from_slice(&(nf as u32).to_le_bytes());
            out.extend_from_slice(&bytes[faces_at + 3..]);
            let mut o = parse_exact(&out).ok()?;
            o.version = 1;
            return (!o.vertices.is_empty() && !o.indices.is_empty()).then_some(o);
        }
    }
    None
}

fn parse_exact(bytes: &[u8]) -> Result<O3d> {
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
            // Zero padding between chunks occurs in some exporters' files; skip it.
            Some(0) => cursor.set_position(cursor.position() + 1),
            // Vertices, faces and materials are complete; anything unrecognised after them is
            // trailing garbage (matrix/bone chunks are optional), so the mesh is kept as is.
            Some(_) => break,
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
        encryption_key: header.stamp.filter(|&k| k != 0xFFFF_FFFF),
        exact_decoded: false,
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

        // Complete core chunks: trailing garbage is ignored instead of rejecting the mesh.
        let o3d = parse(&sample).expect("mesh with trailing garbage still loads");
        assert_eq!(o3d.vertices.len(), 3);
        assert!(o3d.transform.is_some());
    }

    #[test]
    fn test_parse_skips_leading_junk() {
        let mut sample = vec![0x12];
        sample.extend(build_self_authored_v1_sample());
        assert!(parse(&sample).is_ok());
    }

    #[test]
    fn test_parse_repairs_zero_to_space_damage() {
        let damaged: Vec<u8> = build_self_authored_v1_sample()
            .into_iter()
            .map(|b| if b == 0 { 0x20 } else { b })
            .collect();
        let o3d = parse(&damaged).expect("repair");
        assert_eq!(o3d.vertices.len(), 3);
        assert_eq!(o3d.indices, vec![[0, 2, 1]]);
    }

    /// A v1 mesh of 65,539 vertices declares `3` (count modulo 65,536); the reader must find the
    /// real count from the chunk tags that follow.
    #[test]
    fn test_parse_v1_vertex_count_wrapped_at_65536() {
        let nv = 65_539usize;
        let mut b = vec![0x84, 0x19, 0x01, 0x17];
        b.extend_from_slice(&((nv % 65_536) as u16).to_le_bytes());
        for i in 0..nv {
            for f in [i as f32 * 0.001, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0] {
                b.extend_from_slice(&f.to_le_bytes());
            }
        }
        b.push(0x49);
        b.extend_from_slice(&1u16.to_le_bytes());
        for v in [0u16, 1, 2, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.push(0x26);
        b.extend_from_slice(&1u16.to_le_bytes());
        for f in [
            0.8f32, 0.8, 0.8, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 96.0,
        ] {
            b.extend_from_slice(&f.to_le_bytes());
        }
        b.push(0);
        let o = parse(&b).expect("wrapped vertex count is recovered");
        assert_eq!(o.version, 1);
        assert_eq!(o.vertices.len(), nv);
        assert_eq!(o.indices, vec![[0, 1, 2]]);
        assert_eq!(o.materials.len(), 1);
    }
}
