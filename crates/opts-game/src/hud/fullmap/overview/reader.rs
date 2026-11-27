//! The reader thread of the overview: the file source and the walk over every tile.

use super::*;

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
pub(super) fn read_tiles(
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
                        on_course: false,
                        ahead: false,
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
