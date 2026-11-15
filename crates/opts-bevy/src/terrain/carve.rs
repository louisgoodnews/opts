//! The carve pass that cuts the terrain under the splines.

use super::*;

/// Lowers the terrain under drawn ground surfaces so it never covers them
/// (`docs/src/architecture/terrain-carving.md` §4).
///
/// For every surface triangle and every terrain triangle it overlaps (the mesh's own split), the
/// lowest surface height on their overlap minus [`CARVE_EPSILON_M`] caps the terrain triangle's
/// three vertices. A cap that would lower a vertex by more than [`CARVE_COVER_M`] is not applied
/// (buried content). Terrain triangles one cell beyond the tile's border take part too, so a
/// border vertex gets the same caps on both tiles that share it. Heights are only ever lowered;
/// the mesh topology is unchanged.
#[must_use]
pub fn carve_terrain(
    terrain: &TileTerrain,
    surfaces: &[GroundTriangle],
) -> (TileTerrain, CarveStats) {
    let n = terrain.grid.grid_cells as usize;
    let dim = n + 1;
    let mut out = terrain.clone();
    let mut stats = CarveStats::default();
    if n == 0 || terrain.grid.heights.len() < dim * dim {
        return (out, stats);
    }
    let cell = terrain.cell;
    let raw = &terrain.grid.heights;
    let mut cv = Carving::new(n, cell);
    for tri in surfaces {
        cv.add(tri, raw);
    }
    for (k, h) in out.grid.heights.iter_mut().enumerate().take(dim * dim) {
        if cv.cap[k] < *h {
            *h = cv.cap[k];
            stats.lowered += 1;
        } else if cv.guarded[k] {
            stats.guarded += 1;
        }
    }
    // A vertex under a surface that lies lower than the surface is raised to just below it
    // (never above its cap, so the terrain still covers nothing): the terrain triangles that
    // reach out from under the surface then start at the surface's height at its edge, not 10-25
    // cm lower. A bigger gap is content (a bridge, a cutting) and stays.
    for (k, h) in out.grid.heights.iter_mut().enumerate().take(dim * dim) {
        if cv.covered[k]
            && !cv.guarded[k]
            && cv.cap[k].is_finite()
            && *h < cv.cap[k]
            && cv.cap[k] - *h <= ALIGN_MAX_M
        {
            *h = cv.cap[k];
            stats.raised += 1;
        }
    }
    // The terrain beside the surfaces follows their outer edges.
    let outer = align::boundary_edges_out(surfaces);
    stats.aligned = align::align_vertices(
        outer.iter().map(|(a, b, _)| (*a, *b)).collect(),
        &mut out.grid.heights,
        &cv.cap,
        &cv.covered,
        &cv.guarded,
        n,
        cell,
    );
    // … and the terrain just outside every edge is at the edge's height.
    stats.fitted = align::fit_edges(
        &outer,
        &mut out.grid.heights,
        raw,
        &cv.cap,
        &cv.guarded,
        n,
        cell,
    );
    (out, stats)
}
