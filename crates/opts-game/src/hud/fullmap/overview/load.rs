//! Starting the overview's reader and taking its batches; the box of a set of roads.

use super::*;

impl OverviewRoads {
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
