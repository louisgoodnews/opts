//! Unit tests for O3D index buffers: valid 16- and 32-bit indices, out-of-bounds and truncated faces.
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
