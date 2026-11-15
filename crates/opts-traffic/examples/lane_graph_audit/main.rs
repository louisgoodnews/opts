//! Builds the road lane graph of a real map and reports how well the geometric connectivity rule
//! works. Usage: `cargo run -p opts-traffic --example lane_graph_audit -- <map dir>`.

mod components;
mod files;
mod report;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_core::formats::map::{MapManifest, TileGrid};
use opts_traffic::{
    segments_from_tile, signals_from_tile, tile_origin, ConnectParams, LaneGraph, LaneKind,
};

use components::component_sizes;
use files::{find_ci, read_tiles, Files};
use report::{print_conflicts, print_connectivity, print_lights, print_neighbours_and_rules};

fn main() {
    let map_dir = PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let root = map_dir
        .parent()
        .and_then(Path::parent)
        .expect("OMSI root")
        .to_path_buf();
    let mut files = Files {
        root,
        sli: HashMap::new(),
        sco: HashMap::new(),
    };
    let tiles = read_tiles(&map_dir);
    // The tiles' grid: 300 m, or the Mercator rows of a `[worldcoordinates]` map.
    let world_coordinates = find_ci(&map_dir, "global.cfg")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| MapManifest::parse(&b).ok())
        .is_some_and(|m| m.world_coordinates);
    let sample: Vec<_> = tiles.iter().map(|(k, t)| (*k, t.splines.clone())).collect();
    let (grid, _) = TileGrid::measure(&sample, world_coordinates);
    println!("tile grid {grid:?}");
    let mut segments = Vec::new();
    let mut programs = HashMap::new();
    for ((x, y), tile) in &tiles {
        let origin = tile_origin(&grid, *x, *y);
        segments.extend(segments_from_tile(tile, origin, &mut files));
        programs.extend(signals_from_tile(tile, &mut files));
    }
    segments.retain(|s| s.kind == LaneKind::Road);
    let params = ConnectParams::default();
    let t = std::time::Instant::now();
    let graph = LaneGraph::build(&segments, &params);
    let ms = t.elapsed().as_millis();
    let sizes = component_sizes(&graph);
    print_connectivity(&graph, segments.len(), ms);
    print_conflicts(&graph);
    print_lights(&graph, &programs);
    print_neighbours_and_rules(&graph);
    println!(
        "components: {}  largest: {:?}",
        sizes.len(),
        &sizes[..sizes.len().min(5)]
    );
    for tol in [0.25, 0.75, 2.0] {
        let g = LaneGraph::build(
            &segments,
            &ConnectParams {
                position_tolerance: tol,
                ..params
            },
        );
        let joined = g
            .lane_ids()
            .filter(|&l| !g.successors(l).is_empty())
            .count();
        println!("position tolerance {tol:>4} m: {joined} lanes with a successor");
    }
}
