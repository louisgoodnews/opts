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

    // The declared count is not trusted for allocation: each record takes at least
    // MATERIAL_FIXED_BYTES bytes, so the capacity is capped by the bytes that remain.
    let remaining = reader
        .get_ref()
        .len()
        .saturating_sub(reader.position() as usize);
    let mut materials = Vec::with_capacity(material_count.min(remaining / MATERIAL_FIXED_BYTES));

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
            f32::from_le_bytes(fixed_buf[0..4].try_into().unwrap()),
            f32::from_le_bytes(fixed_buf[4..8].try_into().unwrap()),
            f32::from_le_bytes(fixed_buf[8..12].try_into().unwrap()),
            f32::from_le_bytes(fixed_buf[12..16].try_into().unwrap()),
        ];

        let specular = [
            f32::from_le_bytes(fixed_buf[16..20].try_into().unwrap()),
            f32::from_le_bytes(fixed_buf[20..24].try_into().unwrap()),
            f32::from_le_bytes(fixed_buf[24..28].try_into().unwrap()),
        ];

        let emissive = [
            f32::from_le_bytes(fixed_buf[28..32].try_into().unwrap()),
            f32::from_le_bytes(fixed_buf[32..36].try_into().unwrap()),
            f32::from_le_bytes(fixed_buf[36..40].try_into().unwrap()),
        ];

        let power = f32::from_le_bytes(fixed_buf[40..44].try_into().unwrap());

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

            // Legacy add-ons store names in Windows-1252 (umlauts); decode, never reject.
            let tex_name = crate::formats::text::decode_bytes(&name_bytes).to_string();

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

#[cfg(test)]
mod tests;
