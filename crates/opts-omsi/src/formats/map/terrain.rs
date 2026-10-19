//! Parser and data structures for OMSI map terrain elevation grids (`tile_<x>_<y>.map.terrain`).
//!
//! A `.map.terrain` file contains the binary height grid for a single 300 m × 300 m tile.
//! It consists of a 4-byte cell count (typically 60) followed by `(N + 1) * (N + 1)` 32-bit
//! IEEE 754 floating point elevation heights in row-major order with X running fastest.
//!
//! Total file size is always exactly 14,888 bytes (`4 + 61 * 61 * 4`).
//!
//! See specification in `docs/src/formats/map/tiles.md`.

use super::error::MapParseError;

/// Binary height field grid for a map tile (`.map.terrain`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapTerrain {
    /// Number of grid cells per edge (typically 60).
    pub grid_cells: u32,
    /// Elevation heights in meters, row-major `(grid_cells + 1) * (grid_cells + 1)`.
    pub heights: Vec<f32>,
}

impl MapTerrain {
    /// Expected file size in bytes for a standard 60-cell tile.
    pub const STANDARD_FILE_SIZE: usize = 14_888;
    /// Standard vertex dimension per edge (61).
    pub const STANDARD_GRID_DIM: usize = 61;

    /// Parses binary terrain height data.
    ///
    /// # Errors
    /// Returns [`MapParseError::InvalidBinary`] if the byte slice is too small or truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, MapParseError> {
        if bytes.len() < 4 {
            return Err(MapParseError::InvalidBinary {
                format: "terrain",
                detail: format!("file too small: {} bytes", bytes.len()),
            });
        }

        let grid_cells = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let vertices_per_edge = match (grid_cells as usize).checked_add(1) {
            Some(v) => v,
            None => {
                return Err(MapParseError::InvalidBinary {
                    format: "terrain",
                    detail: format!("grid_cells {grid_cells} overflows usize"),
                })
            }
        };
        let expected_vertices = match vertices_per_edge.checked_mul(vertices_per_edge) {
            Some(v) => v,
            None => {
                return Err(MapParseError::InvalidBinary {
                    format: "terrain",
                    detail: format!(
                        "grid dimensions {vertices_per_edge}x{vertices_per_edge} overflow usize"
                    ),
                })
            }
        };
        let expected_bytes = match expected_vertices
            .checked_mul(4)
            .and_then(|b| b.checked_add(4))
        {
            Some(b) => b,
            None => {
                return Err(MapParseError::InvalidBinary {
                    format: "terrain",
                    detail: "byte size calculation overflows usize".to_string(),
                })
            }
        };

        if bytes.len() < expected_bytes {
            return Err(MapParseError::InvalidBinary {
                format: "terrain",
                detail: format!(
                    "truncated terrain data: expected {} bytes for {}x{} grid, found {}",
                    expected_bytes,
                    vertices_per_edge,
                    vertices_per_edge,
                    bytes.len()
                ),
            });
        }

        let mut heights = Vec::with_capacity(expected_vertices);
        for chunk in bytes[4..expected_bytes].chunks_exact(4) {
            let h = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            heights.push(h);
        }

        Ok(Self {
            grid_cells,
            heights,
        })
    }

    /// Queries the elevation at integer vertex coordinates `(x, y)`.
    #[must_use]
    pub fn get_height(&self, x: usize, y: usize) -> Option<f32> {
        let dim = (self.grid_cells as usize) + 1;
        if x < dim && y < dim {
            Some(self.heights[y * dim + x])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_terrain() {
        let mut data = Vec::with_capacity(MapTerrain::STANDARD_FILE_SIZE);
        data.extend_from_slice(&60u32.to_le_bytes());
        for i in 0..(61 * 61) {
            let h = (i as f32) * 0.1;
            data.extend_from_slice(&h.to_le_bytes());
        }
        assert_eq!(data.len(), MapTerrain::STANDARD_FILE_SIZE);

        let terrain = MapTerrain::parse(&data).unwrap();
        assert_eq!(terrain.grid_cells, 60);
        assert_eq!(terrain.heights.len(), 61 * 61);
        assert_eq!(terrain.get_height(0, 0), Some(0.0));
        assert_eq!(terrain.get_height(1, 0), Some(0.1));
        assert_eq!(terrain.get_height(0, 1), Some(6.1));
    }
}
