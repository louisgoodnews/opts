//! Grid measurement from border splines and the per-row step sizes of a `TileGrid`.

use super::*;

impl TileGrid {
    /// Chooses the grid of a map from the splines of several of its tiles (`(tile, splines)`).
    ///
    /// `Mercator` when the map has `[worldcoordinates]` and the measured border steps agree with
    /// it within 0.05 m (median); otherwise `Uniform` with the measured median step (only steps
    /// between 300 and 600 m count), or 300 m when fewer than 5 chains cross a border or the
    /// median is within 0.5 m of 300.
    #[must_use]
    pub fn measure<S: BorderSpline>(
        tiles: &[((i32, i32), Vec<S>)],
        world_coordinates: bool,
    ) -> (Self, GridFit) {
        let pairs = border_pairs(tiles);
        let plausible: Vec<f64> = pairs
            .iter()
            .map(|p| p.step)
            .filter(|s| (300.0..600.0).contains(s))
            .collect();
        let median_step = if plausible.len() >= MIN_PAIRS {
            median(plausible)
        } else {
            None
        };
        let mercator_residual = if world_coordinates && pairs.len() >= MIN_PAIRS {
            median(
                pairs
                    .iter()
                    .map(|p| {
                        let expected = if p.tile.1 == p.next.1 {
                            Self::Mercator.step_ew(p.tile.1)
                        } else {
                            Self::Mercator.step_ns(p.tile.1.min(p.next.1))
                        };
                        (p.step - expected).abs()
                    })
                    .collect(),
            )
        } else {
            None
        };
        let fit = GridFit {
            pairs: pairs.len(),
            median_step,
            mercator_residual,
        };
        if mercator_residual.is_some_and(|r| r <= MERCATOR_AGREEMENT_M) {
            return (Self::Mercator, fit);
        }
        let pitch = match median_step {
            Some(m) if (m - DEFAULT_TILE_SIZE_M).abs() >= 0.5 => m,
            _ => DEFAULT_TILE_SIZE_M,
        };
        (Self::Uniform { pitch }, fit)
    }

    /// Edge length of the tiles of row `ty`, which is also the east-west step between two tiles of
    /// that row (m).
    #[must_use]
    pub fn step_ew(&self, ty: i32) -> f64 {
        match self {
            Self::Uniform { pitch } => *pitch,
            Self::Mercator => mercator_row_width(ty),
        }
    }

    /// North-south step from row `ty` to row `ty + 1` (m).
    #[must_use]
    pub fn step_ns(&self, ty: i32) -> f64 {
        match self {
            Self::Uniform { pitch } => *pitch,
            Self::Mercator => 0.5 * (mercator_row_width(ty) + mercator_row_width(ty + 1)),
        }
    }

    /// Edge length of a tile of row `ty` in its own frame (m); the same as [`Self::step_ew`].
    #[must_use]
    pub fn tile_size(&self, ty: i32) -> f64 {
        self.step_ew(ty)
    }

    /// Terrain cell of row `ty` (a tile has 60 × 60 cells).
    #[must_use]
    pub fn cell(&self, ty: i32) -> f64 {
        self.step_ew(ty) / TERRAIN_CELLS
    }
}
