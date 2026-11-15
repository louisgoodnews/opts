//! Filling the index: a tile's terrain, roads and plates, and dropping a tile again.

use super::*;

impl GroundIndex {
    /// An index of one tile alone (what an object placed on that tile needs).
    #[must_use]
    pub(in super::super) fn for_tile(
        key: (i32, i32),
        origin: (f32, f32),
        terrain: Option<TileTerrain>,
        roads: &[(MapSplineInstance, SplineProfile)],
        start_tile: (i32, i32),
        grid: TileGrid,
    ) -> Self {
        let mut g = Self {
            start_tile,
            grid,
            ..Self::default()
        };
        g.insert_tile(key, origin, terrain, roads);
        g
    }

    /// Registers a tile: its terrain and the surfaces of its road-like splines.
    pub fn insert_tile(
        &mut self,
        key: (i32, i32),
        origin: (f32, f32),
        terrain: Option<TileTerrain>,
        roads: &[(MapSplineInstance, SplineProfile)],
    ) {
        self.remove_tile(key);
        if let Some(t) = terrain {
            self.terrain.insert(key, t);
        }
        for (inst, profile) in roads {
            let surface = std::sync::Arc::new(profile.ground_surface());
            let strips = self.surf.get(&inst.sli_file).cloned().unwrap_or_default();
            let half_width = profile
                .graphical_profiles
                .iter()
                .flat_map(|g| &g.points)
                .map(|p| p.x.abs())
                .fold(0.0_f32, f32::max)
                .max(1.0);
            let steps = ((inst.length_m / SAMPLE_STEP_M).ceil() as usize).clamp(1, 600);
            for k in 0..=steps {
                let p = spline_pose(inst, inst.length_m * k as f32 / steps as f32);
                let s = RoadSample {
                    x: origin.0 + p.x,
                    y: origin.1 + p.y,
                    z: p.z,
                    half_width,
                    heading: p.heading,
                    surface: surface.clone(),
                    mirrored: inst.mirrored,
                    along_m: inst.length_m * k as f32 / steps as f32,
                    strips: strips.clone(),
                };
                self.cells
                    .entry(cell_of(s.x, s.y))
                    .or_default()
                    .push((key, s));
            }
        }
    }

    /// Keeps the strips with a height map of a spline profile, for the tiles registered after.
    pub fn set_surf_strips(
        &mut self,
        sli: &str,
        strips: std::sync::Arc<Vec<super::super::road_surf::TexStrip>>,
    ) {
        self.surf.insert(sli.to_string(), strips);
    }

    /// Registers the textured triangles of a tile's surface plates (M-0146).
    pub fn insert_plate_triangles(
        &mut self,
        key: (i32, i32),
        tris: impl IntoIterator<Item = super::super::plate_surf::SurfTri>,
    ) {
        for t in tris {
            let t = std::sync::Arc::new(t);
            let (lo, hi) = t.bounds();
            let (a, b) = (cell_of(lo.x, lo.y), cell_of(hi.x, hi.y));
            for cx in a.0..=b.0 {
                for cy in a.1..=b.1 {
                    self.plate_tris
                        .entry((cx, cy))
                        .or_default()
                        .push((key, t.clone()));
                }
            }
        }
    }

    /// Registers the surface plates of a tile (see `GroundIndex::plates`).
    pub fn insert_plates(&mut self, key: (i32, i32), plates: impl IntoIterator<Item = Obb>) {
        for p in plates {
            self.plates.insert(key, p);
        }
    }

    /// Forgets a tile.
    pub fn remove_tile(&mut self, key: (i32, i32)) {
        self.plates.remove(&key);
        self.terrain.remove(&key);
        self.plate_tris.retain(|_, v| {
            v.retain(|(k, _)| *k != key);
            !v.is_empty()
        });
        self.cells.retain(|_, v| {
            v.retain(|(k, _)| *k != key);
            !v.is_empty()
        });
    }
}
