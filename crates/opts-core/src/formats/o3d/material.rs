//! Material table decoder for `.o3d` files.
//!
//! Corresponds to `docs/src/formats/binary/o3d.md#0x26--material-chunk` and
//! `docs/src/formats/binary/o3d-chunks.md#0x26-materials`.

use std::io::{Cursor, Read};

use crate::formats::o3d::chunks::{expect_tag, TAG_MATERIAL};
use crate::formats::o3d::error::{O3dParseError, Result};

/// Fixed byte length of the numeric color parameters of a material record (11 * 4 = 44 bytes).
pub const MATERIAL_FIXED_BYTES: usize = 44;

/// A single decoded material specification.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Diffuse RGBA color parameters (`[r, g, b, a]`). Float 3 is opacity/alpha.
    pub diffuse: [f32; 4],
    /// Specular RGB highlight color parameters (`[r, g, b]`).
    pub specular: [f32; 3],
    /// Emissive RGB self-illumination parameters (`[r, g, b]`).
    pub emissive: [f32; 3],
    /// Specular power / shininess exponent.
    pub power: f32,
    /// Texture file references (0 or 1 texture filename string).
    pub textures: Vec<String>,
}

impl Material {
    /// Returns the primary texture filename if one was declared.
    #[must_use]
    pub fn primary_texture(&self) -> Option<&str> {
        self.textures.first().map(String::as_str)
    }
}

/// Decodes the material table chunk (`0x26`).
///
/// # Errors
///
/// Returns [`O3dParseError::Truncated`] if the input stream terminates prematurely.
/// Returns [`O3dParseError::UnknownChunk`] if the leading chunk tag is not `0x26`.
/// Returns [`O3dParseError::InvalidUtf8`] if any texture filename contains invalid UTF-8 bytes.
pub fn parse_material_chunk(reader: &mut Cursor<&[u8]>) -> Result<Vec<Material>> {
    expect_tag(reader, TAG_MATERIAL)?;

    let count_pos = reader.position() as usize;
    let mut count_buf = [0u8; 2];
    reader.read_exact(&mut count_buf).map_err(|_| {
        let actual = reader.get_ref().len().saturating_sub(count_pos);
        O3dParseError::Truncated {
            expected: 2,
            actual,
        }
    })?;
    let material_count = u16::from_le_bytes(count_buf) as usize;

    let mut materials = Vec::with_capacity(material_count);

    for _ in 0..material_count {
        let start_pos = reader.position() as usize;
        let mut fixed_buf = [0u8; MATERIAL_FIXED_BYTES];
        reader.read_exact(&mut fixed_buf).map_err(|_| {
            let actual = reader.get_ref().len().saturating_sub(start_pos);
            O3dParseError::Truncated {
                expected: MATERIAL_FIXED_BYTES,
                actual,
            }
        })?;

        let diffuse = [
            f32::from_le_bytes(read_slice_4(&fixed_buf, 0)),
            f32::from_le_bytes(read_slice_4(&fixed_buf, 4)),
            f32::from_le_bytes(read_slice_4(&fixed_buf, 8)),
            f32::from_le_bytes(read_slice_4(&fixed_buf, 12)),
        ];

        let specular = [
            f32::from_le_bytes(read_slice_4(&fixed_buf, 16)),
            f32::from_le_bytes(read_slice_4(&fixed_buf, 20)),
            f32::from_le_bytes(read_slice_4(&fixed_buf, 24)),
        ];

        let emissive = [
            f32::from_le_bytes(read_slice_4(&fixed_buf, 28)),
            f32::from_le_bytes(read_slice_4(&fixed_buf, 32)),
            f32::from_le_bytes(read_slice_4(&fixed_buf, 36)),
        ];

        let power = f32::from_le_bytes(read_slice_4(&fixed_buf, 40));

        let len_pos = reader.position() as usize;
        let mut name_len_buf = [0u8; 1];
        reader.read_exact(&mut name_len_buf).map_err(|_| {
            let actual = reader.get_ref().len().saturating_sub(len_pos);
            O3dParseError::Truncated {
                expected: 1,
                actual,
            }
        })?;
        let name_len = name_len_buf[0] as usize;

        let str_pos = reader.position() as usize;
        let textures = if name_len > 0 {
            let mut name_bytes = vec![0u8; name_len];
            reader.read_exact(&mut name_bytes).map_err(|_| {
                let actual = reader.get_ref().len().saturating_sub(str_pos);
                O3dParseError::Truncated {
                    expected: name_len,
                    actual,
                }
            })?;

            let tex_name = std::str::from_utf8(&name_bytes)
                .map_err(|_| O3dParseError::InvalidUtf8(str_pos))?
                .to_string();

            vec![tex_name]
        } else {
            Vec::new()
        };

        materials.push(Material {
            diffuse,
            specular,
            emissive,
            power,
            textures,
        });
    }

    Ok(materials)
}

fn read_slice_4(buf: &[u8], offset: usize) -> [u8; 4] {
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&buf[offset..offset + 4]);
    arr
}

#[cfg(test)]
mod tests {
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
    fn test_invalid_utf8_texture_name() {
        let mut bytes = vec![TAG_MATERIAL];
        bytes.extend_from_slice(&1u16.to_le_bytes());
        for _ in 0..11 {
            bytes.extend_from_slice(&0.0f32.to_le_bytes());
        }
        bytes.push(2); // length 2
        bytes.extend_from_slice(&[0xFF, 0xFE]); // invalid UTF-8 bytes

        let mut cursor = Cursor::new(&bytes[..]);
        let err = parse_material_chunk(&mut cursor).unwrap_err();
        assert!(matches!(err, O3dParseError::InvalidUtf8(_)));
    }
}
