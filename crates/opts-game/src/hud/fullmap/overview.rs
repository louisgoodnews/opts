//! The roads of the whole map for the full-size map.
//!
//! The lane graph holds only the resident tiles, so on its own the full-size map would show the
//! neighbourhood of the player and nothing else. When the map is opened for the first time, a
//! background thread lists every tile of the map, reads its splines and scenery paths (the same
//! `[path]` records the traffic graph is built from) and sends the road lanes of each tile to the
//! game; the map draws them for every tile that is not resident, and fills in while the thread
//! works. The thread reads base tiles only (no Chrono overlays, no bent crossings, no terrain), which
//! is exact enough for a 2D overview, and sleeps between tiles so that it does not compete with the
//! streaming of the world.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::OmsiInstall;
use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::{SceneryObject, ScoPath, SplinePath, SplineProfile};
use opts_traffic::{segments_from_tile, LaneKind, PathSource};

use crate::hud::minimap::{may_touch, MapRoad};

/// Pause between two tiles, so the thread stays out of the way of the streaming.
const PAUSE_BETWEEN_TILES: std::time::Duration = std::time::Duration::from_millis(2);

/// The roads of one tile and the box that contains them.
#[derive(Debug, Clone)]
pub struct TileRoads {
    /// The lanes (world frame).
    pub roads: Vec<MapRoad>,
    /// South-west and north-east corner of the box around all lanes, metres.
    pub bounds: (DVec2, DVec2),
}

/// One tile of the listing: its key, its file name and its south-west corner (session frame).
pub type ListedTile = ((i32, i32), String, (f32, f32));

/// One answer of the reader thread.
/// A bus stop object: its ID code, position (east, north) and heading (radians, compass).
pub type StopSite = (u64, DVec2, f64);

type Batch = ((i32, i32), Vec<MapRoad>, Vec<StopSite>);

/// The roads of the whole map as the reader found them.
#[derive(Resource, Default)]
pub struct OverviewRoads {
    tiles: HashMap<(i32, i32), TileRoads>,
    stops: HashMap<u64, (DVec2, f64)>,
    /// Tiles the reader will visit.
    pub total: usize,
    /// Tiles it has finished (including those without roads or without a readable file).
    pub done: usize,
    /// Whether a reader was started this session.
    started: bool,
    rx: Option<Mutex<Receiver<Batch>>>,
    /// Counted up whenever tiles arrive, so the map knows it has new roads to draw.
    pub generation: u64,
}

impl OverviewRoads {
    /// Position and heading of the bus stop object with ID code `id`, once its tile was read.
    #[must_use]
    pub fn stop(&self, id: u64) -> Option<(DVec2, f64)> {
        self.stops.get(&id).copied()
    }

    /// The box around every lane read so far (south-west and north-east corner), if any.
    #[must_use]
    pub fn extent(&self) -> Option<(DVec2, DVec2)> {
        let mut lo = DVec2::splat(f64::MAX);
        let mut hi = DVec2::splat(f64::MIN);
        for tile in self.tiles.values() {
            for r in &tile.roads {
                for p in [
                    r.geometry.point_at(0.0),
                    r.geometry.point_at(r.geometry.length),
                ] {
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
            }
        }
        (lo.x <= hi.x).then_some((lo, hi))
    }

    /// Whether the reader was started.
    #[must_use]
    pub fn started(&self) -> bool {
        self.started
    }

    /// Whether every tile was read (or there is nothing to read).
    #[must_use]
    pub fn complete(&self) -> bool {
        self.started && self.done >= self.total
    }

    /// Percent of the tiles read, 0 to 100.
    #[must_use]
    pub fn percent(&self) -> u32 {
        if self.total == 0 {
            return 100;
        }
        ((self.done * 100) / self.total).min(100) as u32
    }

    /// The roads of tiles for which `skip` is false that may touch the circle.
    pub fn roads_near(
        &self,
        centre: DVec2,
        radius_m: f64,
        skip: impl Fn((i32, i32)) -> bool,
        out: &mut Vec<MapRoad>,
    ) {
        for (key, tile) in &self.tiles {
            let (lo, hi) = tile.bounds;
            if hi.x < centre.x - radius_m
                || lo.x > centre.x + radius_m
                || hi.y < centre.y - radius_m
                || lo.y > centre.y + radius_m
                || skip(*key)
            {
                continue;
            }
            out.extend(
                tile.roads
                    .iter()
                    .filter(|r| may_touch(&r.geometry, centre, radius_m)),
            );
        }
    }

    /// Starts the reader (once per session) for the tiles listed by the load job.
    pub fn start(&mut self, root: PathBuf, map_dir: PathBuf, tiles: Vec<ListedTile>) {
        if self.started {
            return;
        }
        self.started = true;
        self.total = tiles.len();
        let (tx, rx) = channel::<Batch>();
        self.rx = Some(Mutex::new(rx));
        let spawned = std::thread::Builder::new()
            .name("fullmap-overview".into())
            .spawn(move || read_tiles(&root, &map_dir, &tiles, &tx));
        if let Err(e) = spawned {
            tracing::warn!("the overview of the full-size map could not start its reader: {e}");
            self.done = self.total;
            self.rx = None;
        }
    }

    /// Adds the roads of a finished tile (a tile without roads only counts as done).
    pub fn insert(&mut self, key: (i32, i32), roads: Vec<MapRoad>) {
        self.done += 1;
        self.generation += 1;
        if roads.is_empty() {
            return;
        }
        let bounds = bounds_of(&roads);
        self.tiles.insert(key, TileRoads { roads, bounds });
    }

    /// Takes what the reader has sent so far (a few tiles per call).
    pub fn poll(&mut self) {
        let Some(rx) = &self.rx else {
            return;
        };
        let mut got = Vec::new();
        let mut finished = false;
        if let Ok(rx) = rx.lock() {
            for _ in 0..64 {
                match rx.try_recv() {
                    Ok(b) => got.push(b),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }
        for (key, roads, stops) in got {
            self.insert(key, roads);
            self.stops
                .extend(stops.into_iter().map(|(id, p, h)| (id, (p, h))));
        }
        if finished {
            self.rx = None;
            self.done = self.total;
        }
    }
}

/// The box around the lanes, grown by the lane lengths (an arc stays inside its chord's reach).
fn bounds_of(roads: &[MapRoad]) -> (DVec2, DVec2) {
    let mut lo = DVec2::splat(f64::MAX);
    let mut hi = DVec2::splat(f64::MIN);
    for r in roads {
        let reach = r.geometry.length.max(0.0);
        lo = lo.min(r.geometry.start - DVec2::splat(reach));
        hi = hi.max(r.geometry.start + DVec2::splat(reach));
    }
    (lo, hi)
}

/// Reads sli and sco files by their name in the installation, each once.
struct FileSource<'a> {
    install: &'a OmsiInstall,
    sli: HashMap<String, Option<Arc<SplineProfile>>>,
    sco: HashMap<String, Option<Arc<SceneryObject>>>,
}

impl FileSource<'_> {
    /// The parsed scenery object file, read once.
    fn scenery(&mut self, f: &str) -> Option<Arc<SceneryObject>> {
        if !self.sco.contains_key(f) {
            let parsed = self
                .find(f)
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| SceneryObject::parse(&b).ok())
                .map(Arc::new);
            self.sco.insert(f.to_owned(), parsed);
        }
        self.sco.get(f)?.clone()
    }

    fn find(&self, rel: &str) -> Option<PathBuf> {
        self.install
            .find_file(rel)
            .or_else(|| self.install.find_relocated(rel))
    }
}

impl PathSource for FileSource<'_> {
    fn sli_paths(&mut self, f: &str) -> Option<&[SplinePath]> {
        if !self.sli.contains_key(f) {
            let parsed = self
                .find(f)
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| SplineProfile::parse(&b).ok())
                .map(Arc::new);
            self.sli.insert(f.to_owned(), parsed);
        }
        self.sli.get(f)?.as_deref().map(|p| p.paths.as_slice())
    }

    fn sco_paths(&mut self, f: &str) -> Option<&[ScoPath]> {
        self.scenery(f)?;
        self.sco.get(f)?.as_deref().map(|o| o.paths.as_slice())
    }
}

/// The reader thread: every tile, one after the other.
fn read_tiles(
    root: &std::path::Path,
    map_dir: &std::path::Path,
    tiles: &[ListedTile],
    tx: &std::sync::mpsc::Sender<Batch>,
) {
    let install = OmsiInstall::new(root);
    let mut source = FileSource {
        install: &install,
        sli: HashMap::new(),
        sco: HashMap::new(),
    };
    for (key, file, offset) in tiles {
        let (roads, stops) = install
            .find_in(map_dir, file)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| MapTile::parse(&b).ok())
            .map(|tile| {
                let origin = DVec2::new(f64::from(offset.0), f64::from(offset.1));
                let roads = segments_from_tile(&tile, origin, &mut source)
                    .into_iter()
                    .filter(|s| s.kind == LaneKind::Road)
                    .map(|s| MapRoad {
                        geometry: s.geometry,
                        width: s.width,
                    })
                    .collect::<Vec<_>>();
                let stops = tile
                    .objects
                    .iter()
                    .filter(|o| source.scenery(&o.sco_file).is_some_and(|s| s.bus_stop))
                    .map(|o| {
                        let pos =
                            origin + DVec2::new(f64::from(o.position.x), f64::from(o.position.y));
                        (o.id_code, pos, f64::from(o.heading_deg).to_radians())
                    })
                    .collect::<Vec<_>>();
                (roads, stops)
            })
            .unwrap_or_default();
        if tx.send((*key, roads, stops)).is_err() {
            return;
        }
        std::thread::sleep(PAUSE_BETWEEN_TILES);
    }
}
