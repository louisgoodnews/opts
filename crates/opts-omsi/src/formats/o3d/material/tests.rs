//! Unit tests of `material`.

use super::*;

fn create_material_bytes(
    diff: [f32; 4],
    spec: [f32; 3],
    emis: [f32; 3],
    power: f32,
    texture: Option<&str>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
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

    if let Some(tex) = texture {
        bytes.push(tex.len() as u8);
        bytes.extend_from_slice(tex.as_bytes());
    } else {
        bytes.push(0);
    }

    bytes
}

#[test]
fn test_valid_material_table() {
    let mut bytes = vec![TAG_MATERIAL];
    bytes.extend_from_slice(&2u16.to_le_bytes()); // 2 materials
    bytes.extend_from_slice(&create_material_bytes(
        [0.8, 0.8, 0.8, 1.0],
        [0.0, 0.0, 0.0],
        [0.1, 0.1, 0.1],
        96.0,
        Some("bus_body.dds"),
    ));
    bytes.extend_from_slice(&create_material_bytes(
        [1.0, 1.0, 1.0, 0.5],
        [0.5, 0.5, 0.5],
        [0.0, 0.0, 0.0],
        32.0,
        None,
    ));

    let mut cursor = Cursor::new(&bytes[..]);
    let mats = parse_material_chunk(&mut cursor).expect("valid material chunk");
    assert_eq!(mats.len(), 2);

    assert_eq!(mats[0].diffuse, [0.8, 0.8, 0.8, 1.0]);
    assert_eq!(mats[0].specular, [0.0, 0.0, 0.0]);
    assert_eq!(mats[0].emissive, [0.1, 0.1, 0.1]);
    assert_eq!(mats[0].power, 96.0);
    assert_eq!(mats[0].primary_texture(), Some("bus_body.dds"));

    assert_eq!(mats[1].diffuse, [1.0, 1.0, 1.0, 0.5]);
    assert_eq!(mats[1].primary_texture(), None);
    assert_eq!(cursor.position(), bytes.len() as u64);
}

#[test]
fn test_table_shorter_than_declared_count() {
    let mut bytes = vec![TAG_MATERIAL];
    bytes.extend_from_slice(&3u16.to_le_bytes()); // declares 3 materials
    bytes.extend_from_slice(&create_material_bytes(
        [1.0, 1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        10.0,
        None,
    )); // only provides 1

    let mut cursor = Cursor::new(&bytes[..]);
    let err = parse_material_chunk(&mut cursor).unwrap_err();
    assert!(matches!(err, O3dParseError::Truncated { .. }));
}

#[test]
fn test_legacy_encoded_texture_name_is_decoded() {
    let mut bytes = vec![TAG_MATERIAL];
    bytes.extend_from_slice(&1u16.to_le_bytes());
    for _ in 0..11 {
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
    }
    bytes.push(2); // length 2
    bytes.extend_from_slice(&[0xE4, 0xF6]); // "äö" in Windows-1252, invalid UTF-8

    let mut cursor = Cursor::new(&bytes[..]);
    let materials = parse_material_chunk(&mut cursor).expect("legacy encodings must decode");
    assert_eq!(materials[0].textures, vec!["äö".to_string()]);
}
