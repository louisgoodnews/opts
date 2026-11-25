//! Counters, positions and camera parameters of the loaded world.

use bevy::prelude::*;
use opts_core::formats::map::TileGrid;

/// Counters for the diagnostic overlay and the final report.
/// OMSI-frame position (x east, y north, z up, metres) of something in the world.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldPos {
    /// Tile coordinates.
    pub tile: (i32, i32),
    /// Absolute OMSI world position.
    pub world: Vec3,
    /// Position inside the tile (x, y in 0..300).
    pub local: Vec2,
}

/// Camera and vehicle positions for the diagnostic overlay and the log.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PositionReadout {
    /// The 3D camera.
    pub camera: Option<WorldPos>,
    /// The player's vehicle and its heading in degrees (clockwise from north).
    pub vehicle: Option<(WorldPos, f32)>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct WorldStats {
    pub map_name: String,
    pub start_tile: (i32, i32),
    /// Tile grid of the map (where each tile lies relative to the start tile).
    pub grid: TileGrid,
    /// Where the camera and the player's vehicle are (updated every frame).
    pub position: PositionReadout,
    pub tiles_loaded: usize,
    pub tiles_missing: usize,
    pub objects_loaded: usize,
    pub objects_failed: usize,
    /// Objects whose meshes are all scrambled (protected) and therefore not rendered.
    pub objects_protected: usize,
    /// Objects that legitimately have no mesh (path/traffic-light/helper objects).
    pub objects_no_geometry: usize,
    /// `[tree]` billboards spawned (included in `objects_loaded`).
    pub trees_loaded: usize,
    pub splines_loaded: usize,
    pub splines_failed: usize,
    pub scene_vertices: usize,
    pub vehicle_name: String,
    pub vehicle_meshes: usize,
    pub vehicle_vertices: usize,
    pub asset_failures: usize,
    /// Problems recorded by the loading screen (blocking and minor).
    pub load_problems: usize,
    /// Keyed meshes decoded exactly (`o3d::exact`).
    pub meshes_exact: usize,
    /// Tiles with a water surface.
    pub water_tiles: usize,
    pub overlay_visible: bool,
}

/// Vehicle-derived camera parameters handed from vehicle loading to camera setup.
#[derive(Debug, Clone, Copy)]
pub struct CameraSetup {
    pub start: Transform,
    pub chase_distance: f32,
    pub chase_height: f32,
    pub cockpit_offset: Vec3,
}
