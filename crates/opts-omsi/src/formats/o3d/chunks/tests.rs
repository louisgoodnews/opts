//! Unit tests of `chunks`.

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
