//! Terrain shaped to aligned splines (`docs/src/architecture/terrain-carving.md` §10).
//!
//! A spline marked `[spline_terrain_align]` / `[spline_terrain_align_2]` has the terrain under
//! its `[heightprofile]` strips set to their height, however far that is from the authored grid:
//! OMSI's own terrain cache (`*.map.terrain_0.rdy`) holds the result, a re-meshed terrain whose
//! vertices follow the strips' edges (Winsenburg tile -3/-18: the grid under a road 12 m down in a
//! trough is gone, the terrain lies at the road's -11.9 m). The carve only ever moves grid
//! vertices by at most a metre, so such a road stayed under the terrain.
//!
//! [`TerrainCut`] holds the strips' triangles. [`build_cut_terrain_mesh`] clips every terrain
//! triangle against them: the part inside a strip takes the strip's height, the part outside
//! keeps the grid's, and a steep face along the strips' outer edges joins the two. The height
//! function ([`super::TileTerrain::height`]) answers from the strips first.

use super::{
    boundary_edges_out, clip_to_triangle, cross2, terrain_height, GroundTriangle,
    MapSplineInstance, MapTerrain, SplineProfile, CARVE_EPSILON_M,
};
use crate::mesh::smooth_normal_mesh;
use crate::scenery::spline_stations;
use bevy::prelude::*;

mod index;
mod mesh;
pub use mesh::*;

use index::{plane_height, TriIndex};

/// How far the terrain stays under an aligned strip (m): it never covers the road on it.
const CUT_EPSILON_M: f32 = 0.03;
/// A height step along a strip edge below this (m) needs no joining face.
pub(super) const STEP_MIN_M: f32 = 0.02;
/// How far beyond an outer edge another strip is looked for (m): a strip that continues there
/// makes the edge no step (the two halves of a road, a spline's next segment).
const EDGE_PROBE_M: f32 = 0.05;

/// The `[heightprofile]` strips of an aligned spline, extruded over the drawn spline's stations
/// (as [`super::spline_ground_triangles`] does for the drawn surface). `offset` is the corner of
/// the spline's tile in the wanted frame. Profiles without a height profile give nothing.
#[must_use]
pub fn spline_cut_triangles(
    profile: &SplineProfile,
    inst: &MapSplineInstance,
    offset: (f32, f32),
) -> Vec<GroundTriangle> {
    if profile.height_profiles.is_empty() {
        return Vec::new();
    }
    let side = if inst.mirrored { -1.0 } else { 1.0 };
    let stations = spline_stations(inst, offset);
    let at = |k: usize, x: f32, z: f32| {
        let (ex, ny, sz, h, _) = stations[k];
        let x = x * side;
        Vec3::new(ex + h.cos() * x, ny - h.sin() * x, sz + z - CUT_EPSILON_M)
    };
    let mut out = Vec::new();
    for k in 0..stations.len().saturating_sub(1) {
        for hp in &profile.height_profiles {
            let (a, b) = (at(k, hp.x_start, hp.z_start), at(k, hp.x_end, hp.z_end));
            let (c, d) = (
                at(k + 1, hp.x_start, hp.z_start),
                at(k + 1, hp.x_end, hp.z_end),
            );
            out.push([a, b, c]);
            out.push([b, d, c]);
        }
    }
    out
}

/// The aligned strips over one tile, in its frame, and the drawn ground they must stay under.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TerrainCut {
    pub(super) strips: TriIndex,
    /// The drawn ground (surface ribbons, crossing faces): a strip is coarser than the road on
    /// it (one level across a kerbed street), so the terrain takes the lower of the two.
    ground: TriIndex,
    /// The outer edges that are steps: no other strip continues beyond them.
    pub(super) edges: Vec<(Vec3, Vec3)>,
}

impl TerrainCut {
    /// Indexes the aligned `strips` and the drawn `ground` over cells of `cell` metres.
    #[must_use]
    pub fn new(strips: Vec<GroundTriangle>, ground: Vec<GroundTriangle>, cell: f32) -> Self {
        let strips = TriIndex::new(strips, cell);
        let ground = if strips.tris.is_empty() {
            TriIndex::default()
        } else {
            TriIndex::new(ground, cell)
        };
        let mut cut = Self {
            strips,
            ground,
            edges: Vec::new(),
        };
        cut.edges = boundary_edges_out(&cut.strips.tris)
            .into_iter()
            .filter(|(a, b, out)| {
                let mid = ((*a + *b) * 0.5).truncate() + *out * EDGE_PROBE_M;
                cut.height(mid.x, mid.y).is_none()
            })
            .map(|(a, b, _)| (a, b))
            .collect();
        cut
    }

    /// Whether there is no strip.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strips.tris.is_empty()
    }

    /// Number of strip triangles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strips.tris.len()
    }

    /// The terrain's height at `(x, y)` when a strip covers it: the first such strip's, at most
    /// the lowest drawn ground there less the carve's clearance.
    #[must_use]
    pub fn height(&self, x: f32, y: f32) -> Option<f32> {
        let h = self.strips.heights_at(x, y).next()?;
        Some(self.capped(h, x, y))
    }

    /// `h` kept [`CARVE_EPSILON_M`] under the lowest drawn ground at `(x, y)`.
    pub(super) fn capped(&self, h: f32, x: f32, y: f32) -> f32 {
        self.ground
            .heights_at(x, y)
            .fold(h, |h, g| h.min(g - CARVE_EPSILON_M))
    }

    pub(super) fn candidates(&self, i: i32, j: i32) -> &[u32] {
        self.strips.candidates(i, j)
    }
}

#[cfg(test)]
mod tests;
