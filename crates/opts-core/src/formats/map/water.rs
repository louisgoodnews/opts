//! Parser and data structures for OMSI map water level data (`tile_<x>_<y>.map.water`).
//!
//! A `.map.water` file contains binary water elevation levels for a single tile.
//! It consists of a 4-byte header integer (always 1) followed by four 32-bit
//! IEEE 754 floating point water elevation heights (20 bytes total), one per **corner** of
//! the tile in the order south-west, south-east, north-west, north-east (index `x + 2·y`, x east,
//! y north). Neighbouring tiles share the values of their common corners (measured on the corpus:
//! the east edge of a tile equals the west edge of its east neighbour in 355 and 360 of 399
//! varying pairs, the north edge likewise in 390 and 392 of 421), so the surface is continuous.
//!
//! See specification in `docs/src/formats/map/tiles.md`.

use super::error::MapParseError;

/// Binary water elevation definition for a map tile (`.map.water`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapWater {
    /// Water record version/flag (typically 1).
    pub version: u32,
    /// Water level at the tile's corners in metres: south-west, south-east, north-west, north-east.
    pub levels: [f32; 4],
}

impl MapWater {
    /// Expected file size in bytes (20 bytes).
    pub const FILE_SIZE: usize = 20;

    /// Whether the four corners have the same level (a flat lake or sea).
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.levels.iter().all(|l| *l == self.levels[0])
    }

    /// Water level at fractional tile position `(fx, fy)` in `0..=1` (x east, y north), the
    /// bilinear blend of the four corner levels.
    #[must_use]
    pub fn level_at(&self, fx: f32, fy: f32) -> f32 {
        let [sw, se, nw, ne] = self.levels;
        let south = sw + (se - sw) * fx;
        let north = nw + (ne - nw) * fx;
        south + (north - south) * fy
    }

    /// Parses binary tile water elevation data.
    ///
    /// # Errors
    /// Returns [`MapParseError::InvalidBinary`] if the byte slice is not 20 bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, MapParseError> {
        if bytes.len() < Self::FILE_SIZE {
            return Err(MapParseError::InvalidBinary {
                format: "water",
                detail: format!("expected 20 bytes, found {}", bytes.len()),
            });
        }

        let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let l0 = f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let l1 = f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let l2 = f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let l3 = f32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        Ok(Self {
            version,
            levels: [l0, l1, l2, l3],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_blend_bilinearly() {
        let w = MapWater {
            version: 1,
            levels: [0.0, 10.0, 20.0, 30.0],
        };
        assert_eq!(w.level_at(0.0, 0.0), 0.0);
        assert_eq!(w.level_at(1.0, 0.0), 10.0);
        assert_eq!(w.level_at(0.0, 1.0), 20.0);
        assert_eq!(w.level_at(1.0, 1.0), 30.0);
        assert_eq!(w.level_at(0.5, 0.5), 15.0);
        assert!(!w.is_flat());
        assert!(MapWater {
            version: 1,
            levels: [-5.0; 4]
        }
        .is_flat());
    }

    #[test]
    fn test_parse_water() {
        let mut data = Vec::with_capacity(20);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(-5.0f32).to_le_bytes());
        data.extend_from_slice(&(-5.0f32).to_le_bytes());
        data.extend_from_slice(&(-5.0f32).to_le_bytes());
        data.extend_from_slice(&(-5.0f32).to_le_bytes());

        let water = MapWater::parse(&data).unwrap();
        assert_eq!(water.version, 1);
        assert_eq!(water.levels, [-5.0, -5.0, -5.0, -5.0]);
    }
}
