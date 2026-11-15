//! The map of the content test and the helpers that place and bend its objects.

use super::*;

pub(super) struct Map {
    pub(super) install: OmsiInstall,
    pub(super) dir: PathBuf,
    pub(super) files: HashMap<(i32, i32), String>,
    pub(super) grid: TileGrid,
    pub(super) tiles: HashMap<(i32, i32), Option<Arc<MapTile>>>,
    pub(super) sli: HashMap<String, Option<Arc<SplineProfile>>>,
    pub(super) sco: HashMap<String, Option<(PathBuf, Arc<SceneryObject>)>>,
}

impl Map {
    pub(super) fn open(name: &str) -> Option<Self> {
        let root = std::env::var_os("OMSI_PATH").map(PathBuf::from)?;
        let install = OmsiInstall::new(&root);
        let dir = install.find_in(&root.join("maps"), name)?;
        let cfg = install.find_in(&dir, "global.cfg")?;
        let manifest = MapManifest::parse(&std::fs::read(cfg).ok()?).ok()?;
        let files = manifest
            .tiles
            .iter()
            .map(|t| ((t.x, t.y), t.file_name.clone()))
            .collect();
        let mut map = Self {
            install,
            dir,
            files,
            grid: TileGrid::default(),
            tiles: HashMap::new(),
            sli: HashMap::new(),
            sco: HashMap::new(),
        };
        let keys: Vec<(i32, i32)> = map.files.keys().copied().collect();
        let sample: Vec<_> = keys
            .iter()
            .filter_map(|k| Some((*k, map.tile(*k)?.splines.clone())))
            .collect();
        map.grid = TileGrid::measure(&sample, manifest.world_coordinates).0;
        Some(map)
    }

    pub(super) fn tile(&mut self, key: (i32, i32)) -> Option<Arc<MapTile>> {
        if let Some(t) = self.tiles.get(&key) {
            return t.clone();
        }
        let t = self
            .files
            .get(&key)
            .and_then(|f| self.install.find_in(&self.dir, f))
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| MapTile::parse(&b).ok())
            .map(Arc::new);
        self.tiles.insert(key, t.clone());
        t
    }

    pub(super) fn terrain(&self, key: (i32, i32)) -> Option<TileTerrain> {
        let f = self.files.get(&key)?;
        let p = self.install.find_in(&self.dir, &format!("{f}.terrain"))?;
        let t = MapTerrain::parse(&std::fs::read(p).ok()?).ok()?;
        Some(TileTerrain::new(t, self.grid.cell(key.1) as f32))
    }

    pub(super) fn sli(&mut self, rel: &str) -> Option<Arc<SplineProfile>> {
        let install = &self.install;
        self.sli
            .entry(rel.to_lowercase())
            .or_insert_with(|| {
                let p = install.find_file(rel)?;
                SplineProfile::parse(&std::fs::read(p).ok()?)
                    .ok()
                    .map(Arc::new)
            })
            .clone()
    }

    pub(super) fn sco(&mut self, rel: &str) -> Option<(PathBuf, Arc<SceneryObject>)> {
        let install = &self.install;
        self.sco
            .entry(rel.to_lowercase())
            .or_insert_with(|| {
                let p = install.find_file(rel)?;
                let o = SceneryObject::parse(&std::fs::read(&p).ok()?).ok()?;
                Some((p, Arc::new(o)))
            })
            .clone()
    }

    /// The model folder of a `.sco`.
    pub(super) fn model(&self, sco: &Path, file: &str) -> Option<PathBuf> {
        let dir = sco.parent()?;
        let model = self.install.find_in(dir, "model")?;
        self.install.find_in(&model, file)
    }

    /// Height of the raw terrain at a point given in tile `key`'s frame (any tile it falls on).
    pub(super) fn ground_at(
        &self,
        key: (i32, i32),
        p: Vec2,
        cache: &mut HashMap<(i32, i32), Option<TileTerrain>>,
    ) -> Option<f32> {
        let (k, local) = self.grid.tile_at(p.as_dvec2(), key);
        let t = cache.entry(k).or_insert_with(|| self.terrain(k)).as_ref()?;
        Some(terrain_height(
            &t.grid,
            t.cell,
            local.x as f32,
            local.y as f32,
        ))
    }
}

/// Mesh axes of an object (x right, z forward) to its tile's plan frame.
pub(super) fn place(o: &MapObjectInstance, x: f32, forward: f32) -> Vec2 {
    let h = o.heading_deg.to_radians();
    Vec2::new(
        o.position.x + x * h.cos() + forward * h.sin(),
        o.position.y - x * h.sin() + forward * h.cos(),
    )
}

/// The bent meshes and base of a crossing object, or `None` without a deformation.
pub(super) fn bent_meshes(
    map: &Map,
    sco_path: &Path,
    sco: &SceneryObject,
) -> Option<(Vec<opts_core::formats::o3d::O3d>, BaseSurface)> {
    let base_file = sco.crossing_deformation.as_ref()?;
    let base = BaseSurface::from_meshes(
        &load_decoded_meshes(&map.model(sco_path, base_file).expect("base mesh")).expect("decodes"),
    );
    let mut meshes = Vec::new();
    for m in &sco.meshes {
        let Some(p) = map.model(sco_path, &m.file) else {
            continue;
        };
        let Ok(mut models) = load_decoded_meshes(&p) else {
            continue;
        };
        for o in &mut models {
            base.bend_mesh(o);
        }
        meshes.extend(models);
    }
    Some((meshes, base))
}
