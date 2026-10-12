//! A self-authored miniature OMSI installation for loading tests (no OMSI content involved).

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use bevy::math::Vec3;
use opts_core::formats::map::{
    MapEntryPoint, MapManifest, MapObjectInstance, MapTile, MapTileEntry,
};

/// What the mini map looks like.
#[derive(Debug, Clone)]
pub struct MiniMap {
    /// Folder (and map) name.
    pub name: &'static str,
    /// Tiles listed in `global.cfg`.
    pub listed: Vec<(i32, i32)>,
    /// Tiles whose file is written (the rest are missing).
    pub written: Vec<(i32, i32)>,
    /// Scenery objects that name a file that does not exist (failed assets).
    pub dangling_objects: usize,
}

impl MiniMap {
    /// One tile, complete.
    pub fn simple(name: &'static str) -> Self {
        Self {
            name,
            listed: vec![(0, 0)],
            written: vec![(0, 0)],
            dangling_objects: 0,
        }
    }
}

fn object(sco: &str, id: u64, pos: Vec3) -> MapObjectInstance {
    MapObjectInstance {
        flag: 0,
        sco_file: sco.replace('/', "\\"),
        id_code: id,
        position: pos,
        heading_deg: 0.0,
        pitch_deg: Some(0.0),
        roll_deg: Some(0.0),
        string_vars: Vec::new(),
    }
}

/// Writes `root/maps/<name>` with its tiles and returns the map folder.
pub fn write_map(root: &Path, map: &MiniMap) -> PathBuf {
    let dir = root.join("maps").join(map.name);
    std::fs::create_dir_all(&dir).expect("map dir");
    std::fs::create_dir_all(root.join("Vehicles")).expect("vehicles dir");
    let manifest = MapManifest {
        name: map.name.into(),
        version: Some(14),
        tiles: map
            .listed
            .iter()
            .map(|(x, y)| MapTileEntry {
                x: *x,
                y: *y,
                file_name: format!("tile_{x}_{y}.map"),
            })
            .collect(),
        entry_points: vec![MapEntryPoint {
            id: 0,
            id_code: 1,
            position: Vec3::new(20.0, 0.0, 20.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
            name: "Mini start".into(),
        }],
        ..MapManifest::default()
    };
    std::fs::write(
        dir.join("global.cfg"),
        manifest.to_text().expect("global.cfg"),
    )
    .expect("write");
    for (x, y) in &map.written {
        let mut objects = Vec::new();
        if (*x, *y) == map.listed[0] {
            objects.push(object(
                r"Sceneryobjects\Generic\entrypoint_bus.sco",
                1,
                Vec3::new(20.0, 20.0, 0.0),
            ));
            for i in 0..map.dangling_objects {
                objects.push(object(
                    &format!("Sceneryobjects/Mini/missing_{i}.sco"),
                    10 + i as u64,
                    Vec3::new(40.0 + i as f32, 40.0, 0.0),
                ));
            }
        }
        let tile = MapTile {
            version: Some(14),
            objects,
            ..MapTile::default()
        };
        std::fs::write(
            dir.join(format!("tile_{x}_{y}.map")),
            tile.to_text().expect("tile"),
        )
        .expect("write tile");
    }
    dir
}

/// A fresh temporary installation root for `tag`.
pub fn fresh_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("opts_mini_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    root
}
pub mod screens;
