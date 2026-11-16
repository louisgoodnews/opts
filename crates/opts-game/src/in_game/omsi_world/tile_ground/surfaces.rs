//! The drawn ground of a tile: its splines' surface ribbons and aligned strips, kept per tile, and
//! the neighbourhood a terrain carve reads.

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::{spline_cut_triangles, spline_ground_triangles, GroundTriangle};

use super::super::WorldLoadJob;

/// A tile and the eight around it, as key offsets.
const NEIGHBOURHOOD: [(i32, i32); 9] = [
    (0, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// What one tile lays on the terrain, in its own frame.
#[derive(Debug, Default)]
pub(in super::super) struct TileGround {
    /// The drawn ground: its splines' surface ribbons, its crossings' and surface objects' faces.
    surfaces: Vec<GroundTriangle>,
    /// The `[heightprofile]` strips of its terrain-aligned splines
    /// (`docs/src/architecture/terrain-carving.md` §10).
    strips: Vec<GroundTriangle>,
}

/// The drawn ground surfaces (`strips: false`) or the aligned strips (`true`) that can lie over
/// tile `key`'s terrain, in its frame: those of the tile and its eight neighbours (splines are
/// stored on the tile they start in but run on across the border).
pub(super) fn neighbourhood_ground(
    job: &mut WorldLoadJob,
    key: (i32, i32),
    strips: bool,
) -> Vec<GroundTriangle> {
    let mut out = Vec::new();
    for (dx, dy) in NEIGHBOURHOOD {
        let other = (key.0 + dx, key.1 + dy);
        let Some(own) = tile_surfaces(job, other) else {
            continue;
        };
        // The triangles are kept in their own tile's frame; moving them is an addition.
        let o = job.grid.neighbour_offset(key, other);
        let shift = Vec3::new(o.x, o.y, 0.0);
        let tris = if strips { &own.strips } else { &own.surfaces };
        out.extend(tris.iter().map(|t| t.map(|p| p + shift)));
    }
    out
}

/// The drawn ground surfaces of tile `key` in its own frame: the ribbons of its splines and the
/// ground faces of its crossings and surface objects. Computed once per tile and kept
/// (`WorldLoadJob::tile_surfaces`); `None` when the tile has no readable file.
pub(super) fn tile_surfaces(job: &mut WorldLoadJob, key: (i32, i32)) -> Option<Arc<TileGround>> {
    if let Some(s) = job.tile_surfaces.get(&key) {
        return Some(s.clone());
    }
    let splines = job.splines_of(key)?;
    let aligned = job.aligned_of(key);
    let _t = opts_logging::stream_trace::span("terrain.surfaces_build");
    let mut out = TileGround::default();
    for inst in splines.iter() {
        if let Some(profile) = job.sli(&inst.sli_file) {
            out.surfaces
                .extend(spline_ground_triangles(&profile, inst, (0.0, 0.0)));
            if aligned.contains(&inst.id_code) {
                out.strips
                    .extend(spline_cut_triangles(&profile, inst, (0.0, 0.0)));
            }
        }
    }
    // Crossings (bent by their base) and surface objects lay down ground too.
    out.surfaces
        .extend(job.crossing_ground_triangles(key, Vec2::ZERO));
    let out = Arc::new(out);
    job.tile_surfaces.insert(key, out.clone());
    Some(out)
}

/// Forgets the kept surfaces of tiles that no resident tile's carve can need any more.
pub(in super::super) fn prune_tile_surfaces(job: &mut WorldLoadJob) {
    let resident = &job.resident;
    job.tile_surfaces.retain(|k, _| {
        NEIGHBOURHOOD
            .iter()
            .any(|(dx, dy)| resident.contains_key(&(k.0 + dx, k.1 + dy)))
    });
}

/// Terrain stage, part 2: makes sure the surfaces of one of the nine tiles that can lie over
/// tile `key`'s terrain are computed (`index` into [`NEIGHBOURHOOD`]), so that a tile with several
/// new neighbours spreads that work over frames.
pub(in super::super) fn ensure_surfaces(job: &mut WorldLoadJob, key: (i32, i32), index: usize) {
    let (dx, dy) = NEIGHBOURHOOD[index];
    tile_surfaces(job, (key.0 + dx, key.1 + dy));
}

/// Number of tiles whose surfaces the carve of one tile needs.
pub(in super::super) const NEIGHBOURHOOD_LEN: usize = NEIGHBOURHOOD.len();
