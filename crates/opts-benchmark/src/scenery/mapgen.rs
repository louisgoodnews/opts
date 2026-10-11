//! The generated one-tile map: `global.cfg` with one entry point and `tile_0_0.map` holding the
//! entry anchor plus the object under test (`docs/src/architecture/scenery-benchmark.md`, Map
//! generation). Text is produced by `opts_core::formats::map::write`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glam::Vec3;
use opts_core::formats::map::{
    MapEntryPoint, MapManifest, MapObjectInstance, MapTile, MapTileEntry,
};

/// Name of the entry point and the map.
pub const ENTRY_NAME: &str = "Benchmark";
/// Default entry anchor (`[onlyeditor]`, so it is never rendered).
pub const DEFAULT_ANCHOR: &str = r"Sceneryobjects\Generic\entrypoint_bus.sco";
/// Tile-local position of the object under test.
pub const OBJECT_POS: Vec3 = Vec3::new(150.0, 150.0, 0.0);
/// Heading of the object under test; not a multiple of 90° so axis swaps show.
pub const OBJECT_HEADING_DEG: f32 = 37.0;
/// Id code of the object under test (the anchor is 1).
pub const OBJECT_ID: u64 = 2;

/// Writes the files of the generated map into one folder.
#[derive(Debug, Clone)]
pub struct MapWriter {
    /// Map folder.
    pub dir: PathBuf,
    /// Relative path of the entry anchor `.sco`.
    pub anchor: String,
}

impl MapWriter {
    /// Path of the tile file.
    #[must_use]
    pub fn tile_path(&self) -> PathBuf {
        self.dir.join("tile_0_0.map")
    }

    /// Writes `global.cfg` and a tile holding only the anchor.
    ///
    /// # Errors
    /// I/O failures and unwritable values.
    pub fn write_initial(&self) -> Result<()> {
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("create {}", self.dir.display()))?;
        let manifest = MapManifest {
            name: "OPTS Scenery Benchmark".into(),
            version: Some(14),
            tiles: vec![MapTileEntry {
                x: 0,
                y: 0,
                file_name: "tile_0_0.map".into(),
            }],
            entry_points: vec![MapEntryPoint {
                id: 0,
                id_code: 1,
                position: Vec3::new(20.0, 0.0, 20.0),
                direction: Vec3::new(0.0, 0.0, 1.0),
                name: ENTRY_NAME.into(),
            }],
            ..MapManifest::default()
        };
        write(&self.dir.join("global.cfg"), &manifest.to_text()?)?;
        self.write_tile(None)
    }

    /// Rewrites the tile with the anchor and, if given, the object under test.
    ///
    /// # Errors
    /// I/O failures and unwritable values.
    pub fn write_tile(&self, object: Option<(&str, &[String], Placement)>) -> Result<()> {
        let mut objects = vec![instance(
            &self.anchor,
            1,
            Vec3::new(20.0, 20.0, 0.0),
            0.0,
            &[],
        )];
        if let Some((sco, strings, p)) = object {
            objects.push(instance(sco, OBJECT_ID, p.position, p.heading_deg, strings));
        }
        let tile = MapTile {
            version: Some(14),
            objects,
            ..MapTile::default()
        };
        write(&self.tile_path(), &tile.to_text()?)
    }

    /// Copies the map folder (for a `repro/<key>` directory).
    ///
    /// # Errors
    /// I/O failures.
    pub fn copy_to(&self, dest: &Path) -> Result<()> {
        std::fs::create_dir_all(dest)?;
        for name in ["global.cfg", "tile_0_0.map"] {
            std::fs::copy(self.dir.join(name), dest.join(name))?;
        }
        Ok(())
    }
}

/// Position and heading written into the tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Tile-local position.
    pub position: Vec3,
    /// Heading (degrees, clockwise from north).
    pub heading_deg: f32,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            position: OBJECT_POS,
            heading_deg: OBJECT_HEADING_DEG,
        }
    }
}

fn instance(sco: &str, id: u64, pos: Vec3, heading: f32, strings: &[String]) -> MapObjectInstance {
    MapObjectInstance {
        flag: 0,
        sco_file: sco.replace('/', "\\"),
        id_code: id,
        position: pos,
        heading_deg: heading,
        pitch_deg: Some(0.0),
        roll_deg: Some(0.0),
        string_vars: strings.to_vec(),
    }
}

fn write(path: &Path, text: &str) -> Result<()> {
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_map_parses_back() {
        let dir = tempfile::tempdir().unwrap();
        let w = MapWriter {
            dir: dir.path().join("map"),
            anchor: DEFAULT_ANCHOR.into(),
        };
        w.write_initial().unwrap();
        let strings = vec!["OPTS-A1".to_string()];
        w.write_tile(Some((
            "Sceneryobjects/OPTS_Test/box.sco",
            &strings,
            Placement::default(),
        )))
        .unwrap();
        let m = MapManifest::parse(&std::fs::read(w.dir.join("global.cfg")).unwrap()).unwrap();
        assert_eq!(m.entry_points[0].id_code, 1);
        let t = MapTile::parse(&std::fs::read(w.tile_path()).unwrap()).unwrap();
        assert_eq!(t.objects.len(), 2);
        assert_eq!(t.objects[1].sco_file, r"Sceneryobjects\OPTS_Test\box.sco");
        assert_eq!(t.objects[1].heading_deg, OBJECT_HEADING_DEG);
        assert_eq!(t.objects[1].string_vars, strings);
    }
}
