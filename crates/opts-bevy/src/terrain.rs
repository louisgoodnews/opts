//! Terrain height grids (`.map.terrain`) → Bevy meshes, the carve under drawn ground surfaces,
//! and the one height function every consumer samples
//! (`docs/src/architecture/terrain-carving.md`).

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::math::{Vec2, Vec3};
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;
use bevy::render::render_resource::Face;

use opts_core::formats::map::{MapSplineInstance, MapTerrain};
use opts_core::formats::scenery::SplineProfile;

use crate::mesh::smooth_normal_mesh;
use crate::scenery::spline_stations;

/// Default side length of a map tile in metres (`docs/src/formats/map/tiles.md` Q-M01). Maps with
/// `[worldcoordinates]` lay their tiles out on Mercator rows instead
/// (`opts_core::formats::map::TileGrid`); a tile's size and cell come from the session's grid.
pub const TILE_SIZE_M: f32 = 300.0;

/// Default terrain grid cell size (60 cells per default tile).
pub const TERRAIN_CELL_M: f32 = TILE_SIZE_M / 60.0;

/// Clearance the carve leaves between a drawn ground surface and the terrain under it (m).
pub const CARVE_EPSILON_M: f32 = 0.03;
/// Deepest a vertex is lowered for one surface (m): terrain further above a surface is buried
/// content (a tunnel, a river bed, stale paving) and stays as authored.
pub const CARVE_COVER_M: f32 = 1.0;

mod align;
pub use align::{align_band, boundary_edges, boundary_edges_out, ALIGN_MAX_M};
pub mod cut;
pub use cut::{
    build_cut_terrain_mesh, build_cut_terrain_mesh_with, spline_cut_triangles, TerrainCut,
};

mod mesh;
pub use mesh::*;

mod normals;
pub use normals::*;

mod spline;
pub use spline::*;

mod clip;
use clip::*;

mod carving;
use carving::*;

mod carve;
pub use carve::*;

#[must_use]
pub fn terrain_material(texture: Option<Handle<Image>>) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: texture,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        cull_mode: Some(Face::Back),
        ..Default::default()
    }
}

#[cfg(test)]
pub(super) mod tests;
