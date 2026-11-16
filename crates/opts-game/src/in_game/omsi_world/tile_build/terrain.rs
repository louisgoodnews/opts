//! The terrain steps of a tile build: the terrain grid and the neighbours' ground surfaces.

use super::*;

/// Reads the tile's terrain grid and builds its ground inputs.
pub(super) fn terrain_read_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    ground_materials: Option<&mut Assets<GroundMaterial>>,
) {
    let _t = opts_logging::stream_trace::span("tile.terrain");
    b.terrain_input = load_terrain(job, meshes, materials, images, ground_materials, b.key);
    b.phase = Phase::TerrainSurfaces;
}

/// Makes the surface data of one neighbouring tile available (one per call).
pub(super) fn terrain_surfaces_step(b: &mut TileBuild, job: &mut WorldLoadJob) {
    let _t = opts_logging::stream_trace::span("tile.terrain");
    // A tile without a grid needs no surfaces.
    if b.terrain_input.is_none() || b.next_neighbour >= NEIGHBOURHOOD_LEN {
        b.phase = Phase::Terrain;
    } else {
        ensure_surfaces(job, b.key, b.next_neighbour);
        b.next_neighbour += 1;
    }
}
