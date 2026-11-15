//! The map's files as the traffic crate reads them: case-insensitive lookup, tiles and paths.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::{
    SceneryObject, ScoPath, ScoTrafficLightGroup, SplinePath, SplineProfile,
};
use opts_traffic::PathSource;

pub(crate) fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    Some(cur)
}

pub(crate) struct Files {
    pub(crate) root: PathBuf,
    pub(crate) sli: HashMap<String, Option<Vec<SplinePath>>>,
    pub(crate) sco: HashMap<String, Option<SceneryObject>>,
}

impl PathSource for Files {
    fn sli_paths(&mut self, f: &str) -> Option<&[SplinePath]> {
        let root = &self.root;
        self.sli
            .entry(f.to_owned())
            .or_insert_with(|| {
                let bytes = std::fs::read(find_ci(root, f)?).ok()?;
                SplineProfile::parse(&bytes).ok().map(|p| p.paths)
            })
            .as_deref()
    }

    fn sco_paths(&mut self, f: &str) -> Option<&[ScoPath]> {
        let root = &self.root;
        self.sco
            .entry(f.to_owned())
            .or_insert_with(|| {
                let bytes = std::fs::read(find_ci(root, f)?).ok()?;
                SceneryObject::parse(&bytes).ok()
            })
            .as_ref()
            .map(|o| o.paths.as_slice())
    }

    fn sco_lights(&mut self, f: &str) -> Option<&[ScoTrafficLightGroup]> {
        self.sco_paths(f)?;
        self.sco
            .get(f)?
            .as_ref()
            .map(|o| o.traffic_light_groups.as_slice())
    }
}

pub(crate) type Programs = HashMap<opts_traffic::SignalKey, opts_traffic::SignalProgram>;

/// The tiles of the map folder (`tile_<x>_<y>.map`).
pub(crate) fn read_tiles(map_dir: &Path) -> Vec<((i32, i32), MapTile)> {
    let mut tiles = Vec::new();
    for e in std::fs::read_dir(map_dir).expect("map dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(rest) = name
            .strip_prefix("tile_")
            .and_then(|r| r.strip_suffix(".map"))
        else {
            continue;
        };
        let mut it = rest.split('_');
        let (Some(Ok(x)), Some(Ok(y))) = (
            it.next().map(str::parse::<i32>),
            it.next().map(str::parse::<i32>),
        ) else {
            continue;
        };
        let Some(tile) = std::fs::read(e.path())
            .ok()
            .and_then(|b| MapTile::parse(&b).ok())
        else {
            continue;
        };
        tiles.push(((x, y), tile));
    }
    tiles
}
