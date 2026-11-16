//! The tiles of a map (read on first use) and the road lanes of the 3 x 3 tiles around one.

use std::collections::HashMap;
use std::path::Path;

use super::{MapTile, TileGrid};
use opts_traffic::{segments_from_tile, tile_origin, ConnectParams, LaneGraph, LaneKind};

use crate::files::{read, Files};

/// The tiles of a map, read on first use.
pub(crate) struct Tiles<'a> {
    pub(crate) map_dir: &'a Path,
    pub(crate) keys: HashMap<(i32, i32), String>,
    pub(crate) cache: HashMap<(i32, i32), Option<MapTile>>,
}

impl Tiles<'_> {
    pub(crate) fn load(&mut self, k: (i32, i32)) -> Option<MapTile> {
        let (keys, dir) = (&self.keys, self.map_dir);
        self.cache
            .entry(k)
            .or_insert_with(|| {
                keys.get(&k)
                    .and_then(|n| read(dir, n))
                    .and_then(|b| MapTile::parse(&b).ok())
            })
            .clone()
    }
}

/// The road lanes of the 3 x 3 tiles around tile `key`.
pub(crate) fn road_graph(
    tiles: &mut Tiles,
    grid: &TileGrid,
    files: &mut Files,
    key: (i32, i32),
) -> LaneGraph {
    let mut segs = Vec::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            let k = (key.0 + dx, key.1 + dy);
            if let Some(tile) = tiles.load(k) {
                segs.extend(segments_from_tile(
                    &tile,
                    tile_origin(grid, k.0, k.1),
                    files,
                ));
            }
        }
    }
    segs.retain(|s| s.kind == LaneKind::Road);
    LaneGraph::build(&segs, &ConnectParams::default())
}
