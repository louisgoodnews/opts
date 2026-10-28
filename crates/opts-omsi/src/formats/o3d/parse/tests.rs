//! Unit tests for `super` (`parse.rs`).

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
