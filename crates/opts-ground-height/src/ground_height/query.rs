//! Asking the index: the terrain and road height under a point, and the road texture's unevenness.

use super::*;

impl GroundIndex {
    /// The fine unevenness of the road texture under a tyre at `(x, y)` (m, ±2 cm; M-0146): from
    /// the nearest road sample at about `near_z`; off the splines that of plain asphalt.
    #[must_use]
    pub fn unevenness_at(&self, x: f32, y: f32, near_z: f32) -> f32 {
        let (cx, cy) = cell_of(x, y);
        let mut best: Option<(f32, &RoadSample)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for (_, s) in self.cells.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    let d = (s.x - x).hypot(s.y - y);
                    if d <= s.half_width + SAMPLE_STEP_M * 0.5
                        && (s.surface_z(x, y) - near_z).abs() < 0.6
                        && best.is_none_or(|(bd, _)| d < bd)
                    {
                        best = Some((d, s));
                    }
                }
            }
        }
        match best {
            Some((_, s)) => s.unevenness(x, y),
            // A surface plate (junction, square): the texture of its triangle under the wheel,
            // the one nearest `near_z`; the terrain: plain asphalt, laid in metres.
            None => {
                let here = Vec2::new(x, y);
                self.plate_tris
                    .get(&cell_of(x, y))
                    .into_iter()
                    .flatten()
                    .filter_map(|(_, t)| t.at(here))
                    .filter(|(z, _)| (z - near_z).abs() < 0.6)
                    .min_by(|a, b| (a.0 - near_z).abs().total_cmp(&(b.0 - near_z).abs()))
                    .map_or_else(
                        || opts_road_surf::plain_unevenness(x, y),
                        |(_, u)| u,
                    )
            }
        }
    }

    /// Terrain height at a session-frame position.
    #[must_use]
    pub fn terrain_at(&self, x: f32, y: f32) -> Option<f32> {
        self.terrain.height_at(x, y)
    }

    /// Height of the road at `(x, y)` when the point lies on one (`near_z` picks among overlapping
    /// surfaces), `None` on bare terrain.
    #[must_use]
    pub fn road_height_at(&self, x: f32, y: f32, near_z: f32) -> Option<f32> {
        let h = self.height_at(x, y, near_z)?;
        let t = self.terrain_at(x, y);
        t.is_none_or(|t| (h - t).abs() > 1e-4).then_some(h)
    }

    /// Height of the surface the vehicle stands on at `(x, y)`: the road when the point lies on
    /// one (the sample whose height is closest to `near_z` where several overlap, e.g. bridges),
    /// else the terrain.
    #[must_use]
    pub fn height_at(&self, x: f32, y: f32, near_z: f32) -> Option<f32> {
        let (cx, cy) = cell_of(x, y);
        // Road samples the point lies on: (distance, height).
        let mut on_road: Vec<(f32, f32)> = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for (_, s) in self.cells.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    let d = (s.x - x).hypot(s.y - y);
                    // Samples are 2 m apart: a point is on the road within its half width plus
                    // half a sample step.
                    if d <= s.half_width + SAMPLE_STEP_M * 0.5 {
                        on_road.push((d, s.surface_z(x, y)));
                    }
                }
            }
        }
        // Of overlapping surfaces (bridges) the one closest to `near_z`; its height is blended
        // from the samples around the point, so the height is continuous along the road instead
        // of stepping from sample to sample (which made the vehicle pitch and bob).
        // The plates of `[surface]` objects are road layers as well.
        let here = Vec2::new(x, y);
        for plate in self.plates.query(here, 0.5) {
            if plate.contains(here) {
                on_road.push((0.0, plate.z_max));
            }
        }
        let best = on_road
            .iter()
            .map(|(_, z)| *z)
            .min_by(|a, b| (a - near_z).abs().total_cmp(&(b - near_z).abs()));
        // The terrain is a layer too. A layer well above the vehicle is a deck over it (a railway
        // bridge above a bus station apron that has no road of its own): the vehicle stands on
        // the terrain, it does not climb. A layer slightly above (a street plate a few
        // decimetres over the terrain) is the ground. A layer below the vehicle loses to the
        // terrain only when the terrain is clearly nearer.
        if let (Some(z0), Some(t)) = (best, self.terrain_at(x, y)) {
            let rise = z0 - near_z;
            let use_terrain = (rise > DECK_GAP_M && (t - near_z).abs() < rise)
                || (rise < 0.0 && (t - near_z).abs() + TERRAIN_LAYER_MARGIN_M < -rise);
            if use_terrain {
                return Some(t);
            }
        }
        match best {
            Some(z0) => {
                let (mut num, mut den) = (0.0_f32, 0.0_f32);
                for (d, z) in on_road.iter().filter(|(_, z)| (z - z0).abs() < 0.6) {
                    // Only the samples nearest to the point along the road count.
                    let w = 1.0 / (d + 0.5).powi(3);
                    num += w * z;
                    den += w;
                }
                Some(num / den)
            }
            None => self.terrain_at(x, y),
        }
    }
}
