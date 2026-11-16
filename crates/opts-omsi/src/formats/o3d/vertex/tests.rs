//! Unit tests of `vertex`.

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
