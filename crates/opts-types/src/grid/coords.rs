//! Tile origins, offsets and coordinate conversions of a `TileGrid`.

use super::*;

impl TileGrid {
    /// Northing of row `ty`'s origins relative to row `ay` (m).
    fn row_offset(&self, ty: i32, ay: i32) -> f64 {
        match self {
            Self::Uniform { pitch } => f64::from(ty - ay) * pitch,
            Self::Mercator if (ty - ay).abs() <= 64 => {
                if ty >= ay {
                    (ay..ty).map(|r| self.step_ns(r)).sum()
                } else {
                    -(ty..ay).map(|r| self.step_ns(r)).sum::<f64>()
                }
            }
            Self::Mercator => mercator_rows_from_zero(ty) - mercator_rows_from_zero(ay),
        }
    }

    /// South-west corner of tile `key` relative to the corner of tile `anchor` (m).
    ///
    /// `x = (tx − ax) · step_ew(ty)`: tiles of one column share their x origin (zero shear);
    /// `y` sums the north-south steps from the anchor row.
    #[must_use]
    pub fn origin(&self, key: (i32, i32), anchor: (i32, i32)) -> DVec2 {
        DVec2::new(
            f64::from(key.0 - anchor.0) * self.step_ew(key.1),
            self.row_offset(key.1, anchor.1),
        )
    }

    /// [`Self::origin`] as the `f32` offset of the session frame.
    #[must_use]
    pub fn offset(&self, key: (i32, i32), anchor: (i32, i32)) -> Vec2 {
        self.origin(key, anchor).as_vec2()
    }

    /// Offset between two tiles: the corner of `other` in the frame of `key` (`f32`).
    #[must_use]
    pub fn neighbour_offset(&self, key: (i32, i32), other: (i32, i32)) -> Vec2 {
        self.origin(other, key).as_vec2()
    }

    /// The tile a point lies on and the point in that tile's frame; `local` is relative to the
    /// anchor tile's corner.
    #[must_use]
    pub fn tile_at(&self, local: DVec2, anchor: (i32, i32)) -> ((i32, i32), DVec2) {
        let ty = match self {
            Self::Uniform { pitch } => anchor.1 + (local.y / pitch).floor() as i32,
            Self::Mercator => {
                // Jump by the local row step until the row is found (a few rounds even from
                // row 0), then settle on the row whose band holds the point.
                let mut ty = anchor.1;
                for _ in 0..32 {
                    let jump = ((local.y - self.row_offset(ty, anchor.1)) / self.step_ns(ty))
                        .floor() as i32;
                    if jump == 0 {
                        break;
                    }
                    ty += jump;
                }
                while local.y < self.row_offset(ty, anchor.1) {
                    ty -= 1;
                }
                while local.y >= self.row_offset(ty + 1, anchor.1) {
                    ty += 1;
                }
                ty
            }
        };
        let w = self.step_ew(ty);
        let tx = anchor.0 + (local.x / w).floor() as i32;
        let key = (tx, ty);
        (key, local - self.origin(key, anchor))
    }

    /// [`Self::tile_at`] for an `f32` session-frame point, returning only the tile.
    #[must_use]
    pub fn key_at(&self, local: Vec2, anchor: (i32, i32)) -> (i32, i32) {
        self.tile_at(local.as_dvec2(), anchor).0
    }

    /// Absolute position of a point of tile `key` (its frame relative to tile (0, 0)), as the
    /// control protocol speaks it. On a uniform grid this is `key · pitch + local`.
    #[must_use]
    pub fn absolute(&self, key: (i32, i32), local: DVec2) -> DVec2 {
        self.origin(key, (0, 0)) + local
    }

    /// Session-frame position of an absolute position (see [`Self::absolute`]), for a session
    /// anchored at `anchor`.
    #[must_use]
    pub fn absolute_to_local(&self, abs: DVec2, anchor: (i32, i32)) -> DVec2 {
        let (key, local) = self.tile_at(abs, (0, 0));
        self.origin(key, anchor) + local
    }

    /// Absolute position of a session-frame position (see [`Self::absolute`]).
    #[must_use]
    pub fn local_to_absolute(&self, local: DVec2, anchor: (i32, i32)) -> DVec2 {
        let (key, in_tile) = self.tile_at(local, anchor);
        self.absolute(key, in_tile)
    }
}
