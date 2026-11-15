//! Content test of the terrain carve and the crossing bend on Berlin-Spandau
//! (`docs/src/architecture/terrain-carving.md` §7, `docs/src/meta/berlin-spandau-2026-10-17.md`
//! System 2). Skipped unless `OMSI_PATH` points at an installation with that map.
//!
//! * The tile with the old embankment pits (2402/11279) is carved without lowering any vertex
//!   by more than the 1 m guard, and its Damm1 splines leave the ground beside their crest alone.
//! * The crossings with `[crossing_heightdeformation]`, bent over their base, lie on or above the
//!   terrain, and their `[splinehelper]` arms meet the spline ends within 1 cm in height (one
//!   arm of the map within 2 cm).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::math::Vec2;
use opts_bevy::install::OmsiInstall;
use opts_bevy::scenery::spline_pose;
use opts_bevy::{
    carve_terrain, load_decoded_meshes, spline_ground_triangles, terrain_height, GroundTriangle,
    TileTerrain, CARVE_COVER_M,
};
use opts_core::formats::map::{MapManifest, MapObjectInstance, MapTerrain, MapTile, TileGrid};
use opts_core::formats::scenery::{
    crossing_ground_faces, BaseSurface, SceneryObject, SplineProfile,
};

#[path = "terrain_carve_content/support.rs"]
mod support;
use support::*;

#[path = "terrain_carve_content/bent.rs"]
mod bent;

#[test]
fn spandau_carve_digs_no_pits() {
    let Some(mut map) = Map::open("Berlin-Spandau") else {
        eprintln!("OMSI_PATH not set or Berlin-Spandau not installed; skipping");
        return;
    };
    assert_eq!(map.grid, TileGrid::Mercator);
    let key = (2402, 11279);
    let raw = map.terrain(key).expect("terrain");
    let mut surfaces: Vec<GroundTriangle> = Vec::new();
    let mut damm: Vec<(Vec2, f32)> = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let other = (key.0 + dx, key.1 + dy);
            let Some(tile) = map.tile(other) else {
                continue;
            };
            let o = map.grid.neighbour_offset(key, other);
            for s in &tile.splines {
                let Some(p) = map.sli(&s.sli_file) else {
                    continue;
                };
                surfaces.extend(spline_ground_triangles(&p, s, (o.x, o.y)));
                if other == key && s.sli_file.to_lowercase().contains("damm1") {
                    let mid = spline_pose(s, s.length_m * 0.5);
                    damm.push((Vec2::new(mid.x, mid.y), mid.heading));
                }
            }
        }
    }
    let (carved, stats) = carve_terrain(&raw, &surfaces);
    let n = raw.grid.heights.len();
    let worst = (0..n)
        .map(|k| raw.grid.heights[k] - carved.grid.heights[k])
        .fold(0.0_f32, f32::max);
    eprintln!(
        "2402/11279: lowered {}, guarded {}, deepest {worst:.3} m",
        stats.lowered, stats.guarded
    );
    assert!(stats.lowered > 0);
    // The alignment pulls vertices beside the surfaces to their outer edges: up or down, never by
    // more than ALIGN_MAX_M beyond the carve, and never above the carve's caps (the test above
    // that the terrain never covers a road holds for the same input).
    eprintln!("aligned {}", stats.aligned);
    assert!(
        stats.aligned > 0,
        "the tile has roads with terrain beside them"
    );
    let raised = (0..n)
        .map(|k| carved.grid.heights[k] - raw.grid.heights[k])
        .fold(0.0_f32, f32::max);
    assert!(
        raised <= opts_bevy::terrain::ALIGN_MAX_M + 1e-4,
        "a vertex is raised by {raised} m"
    );
    assert!(
        worst <= CARVE_COVER_M + opts_bevy::terrain::ALIGN_MAX_M + 1e-4,
        "a vertex is lowered by {worst} m"
    );
    // Beside the Damm1 embankments (15-30 m from the axis, the old pit) nothing changes.
    assert!(!damm.is_empty(), "the tile has Damm1 splines");
    for (mid, h) in &damm {
        let right = Vec2::new(h.cos(), -h.sin());
        for side in [-1.0, 1.0] {
            for d in [15.0, 20.0, 30.0] {
                let p = *mid + right * side * d;
                if !(0.0..360.0).contains(&p.x) || !(0.0..360.0).contains(&p.y) {
                    continue;
                }
                let (a, b) = (raw.height(p.x, p.y), carved.height(p.x, p.y));
                assert!(a - b < 0.5, "flank at {p} lowered from {a} to {b}");
            }
        }
    }
}

#[path = "terrain_carve_content/crossing_tally.rs"]
mod crossing_tally;
use crossing_tally::Tally;

#[path = "terrain_carve_content/edge_fit.rs"]
mod edge_fit;
