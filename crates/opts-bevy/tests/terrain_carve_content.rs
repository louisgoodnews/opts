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

struct Map {
    install: OmsiInstall,
    dir: PathBuf,
    files: HashMap<(i32, i32), String>,
    grid: TileGrid,
    tiles: HashMap<(i32, i32), Option<Arc<MapTile>>>,
    sli: HashMap<String, Option<Arc<SplineProfile>>>,
    sco: HashMap<String, Option<(PathBuf, Arc<SceneryObject>)>>,
}

impl Map {
    fn open(name: &str) -> Option<Self> {
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

    fn tile(&mut self, key: (i32, i32)) -> Option<Arc<MapTile>> {
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

    fn terrain(&self, key: (i32, i32)) -> Option<TileTerrain> {
        let f = self.files.get(&key)?;
        let p = self.install.find_in(&self.dir, &format!("{f}.terrain"))?;
        let t = MapTerrain::parse(&std::fs::read(p).ok()?).ok()?;
        Some(TileTerrain::new(t, self.grid.cell(key.1) as f32))
    }

    fn sli(&mut self, rel: &str) -> Option<Arc<SplineProfile>> {
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

    fn sco(&mut self, rel: &str) -> Option<(PathBuf, Arc<SceneryObject>)> {
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
    fn model(&self, sco: &Path, file: &str) -> Option<PathBuf> {
        let dir = sco.parent()?;
        let model = self.install.find_in(dir, "model")?;
        self.install.find_in(&model, file)
    }

    /// Height of the raw terrain at a point given in tile `key`'s frame (any tile it falls on).
    fn ground_at(
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
fn place(o: &MapObjectInstance, x: f32, forward: f32) -> Vec2 {
    let h = o.heading_deg.to_radians();
    Vec2::new(
        o.position.x + x * h.cos() + forward * h.sin(),
        o.position.y - x * h.sin() + forward * h.cos(),
    )
}

/// The bent meshes and base of a crossing object, or `None` without a deformation.
fn bent_meshes(
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

#[test]
fn spandau_bent_crossings_lie_on_the_terrain_and_meet_their_arms() {
    let Some(mut map) = Map::open("Berlin-Spandau") else {
        eprintln!("OMSI_PATH not set or Berlin-Spandau not installed; skipping");
        return;
    };
    let keys: Vec<(i32, i32)> = map.files.keys().copied().collect();
    let mut terrain = HashMap::new();
    let (mut placements, mut area, mut below) = (0, 0.0_f64, 0.0_f64);
    let (mut arms, mut arm_misses) = (0, Vec::new());
    for key in keys {
        let Some(tile) = map.tile(key) else { continue };
        for o in &tile.objects {
            let Some((path, sco)) = map.sco(&o.sco_file) else {
                continue;
            };
            let Some((meshes, base)) = bent_meshes(&map, &path, &sco) else {
                continue;
            };
            placements += 1;
            // Ground faces: centroid and three inner points, area-weighted.
            for f in crossing_ground_faces(&meshes, Some(&base)) {
                let a = 0.5 * (f[1] - f[0]).cross(f[2] - f[0]).length();
                let c = (f[0] + f[1] + f[2]) / 3.0;
                for q in [
                    c,
                    (2.0 * c + f[0]) / 3.0,
                    (2.0 * c + f[1]) / 3.0,
                    (2.0 * c + f[2]) / 3.0,
                ] {
                    let Some(g) = map.ground_at(key, place(o, q.x, q.z), &mut terrain) else {
                        continue;
                    };
                    area += f64::from(a) / 4.0;
                    if g - (o.position.z + q.y) > 0.02 {
                        below += f64::from(a) / 4.0;
                    }
                }
            }
            // Arms against the spline ends of the tile and its neighbours.
            let bent = sco.bent(&base);
            for h in &bent.spline_helpers {
                let at = place(o, h.position.x, h.position.y);
                let z = o.position.z + h.position.z;
                let mut best: Option<(f32, f32)> = None;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let other = (key.0 + dx, key.1 + dy);
                        let Some(t) = map.tile(other) else { continue };
                        let off = map.grid.neighbour_offset(key, other);
                        for s in &t.splines {
                            for p in [spline_pose(s, 0.0), spline_pose(s, s.length_m)] {
                                let d = (Vec2::new(p.x, p.y) + off).distance(at);
                                if best.is_none_or(|(bd, _)| d < bd) {
                                    best = Some((d, p.z));
                                }
                            }
                        }
                    }
                }
                if let Some((d, sz)) = best.filter(|(d, _)| *d < 0.5) {
                    arms += 1;
                    if (sz - z).abs() > 0.01 {
                        arm_misses.push((o.sco_file.clone(), d, sz - z));
                    }
                }
            }
        }
    }
    let share = 100.0 * below / area.max(1.0);
    eprintln!(
        "bent placements {placements}, ground {area:.0} m2, below terrain {share:.3} %, arms {arms}, height misses {}",
        arm_misses.len()
    );
    assert_eq!(placements, 11);
    assert!(
        share < 0.1,
        "{share:.3} % of the bent crossing ground lies below the terrain"
    );
    assert!(arms > 0);
    // 38 of 39 arms meet their spline within 1 cm; one (`Einm_SpandauerD_WiesenD`) is 1.3 cm
    // off. None may be further than 2 cm.
    assert!(
        arm_misses.len() <= 1,
        "arms off their spline: {arm_misses:?}"
    );
    assert!(
        arm_misses.iter().all(|(_, _, dz)| dz.abs() < 0.02),
        "arms off their spline: {arm_misses:?}"
    );
}

/// How far the terrain beside the outer edges of the road surfaces lies from the edge height
/// after the carve and the alignment, on the tile of Berlin-Spandau's Westerwaldstraße start
/// point (the project lead's report 2026-10-24: the terrain must fit the outer edges).
#[test]
fn spandau_terrain_beside_the_surface_edges_fits_their_height() {
    let Some(mut map) = Map::open("Berlin-Spandau") else {
        eprintln!("OMSI_PATH not set or Berlin-Spandau not installed; skipping");
        return;
    };
    let key = (2402, 11279);
    let raw = map.terrain(key).expect("terrain");
    let mut surfaces: Vec<GroundTriangle> = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let other = (key.0 + dx, key.1 + dy);
            let Some(tile) = map.tile(other) else {
                continue;
            };
            let o = map.grid.neighbour_offset(key, other);
            for s in &tile.splines {
                if let Some(p) = map.sli(&s.sli_file) {
                    surfaces.extend(spline_ground_triangles(&p, s, (o.x, o.y)));
                }
            }
        }
    }
    let (carved, _) = carve_terrain(&raw, &surfaces);
    // Outer edges with the side that is outside (away from the triangle's third corner).
    type P = (i64, i64, i64);
    let k = |v: bevy::math::Vec3| -> P {
        (
            (v.x * 100.0).round() as i64,
            (v.y * 100.0).round() as i64,
            (v.z * 100.0).round() as i64,
        )
    };
    let mut count: HashMap<(P, P), (u8, bevy::math::Vec3, bevy::math::Vec3, bevy::math::Vec3)> =
        HashMap::new();
    for t in &surfaces {
        for e in 0..3 {
            let (a, b, c) = (t[e], t[(e + 1) % 3], t[(e + 2) % 3]);
            let (ka, kb) = (k(a), k(b));
            if ka == kb {
                continue;
            }
            let key2 = if ka < kb { (ka, kb) } else { (kb, ka) };
            let entry = count.entry(key2).or_insert((0, a, b, c));
            entry.0 = entry.0.saturating_add(1);
        }
    }
    let mut gaps: Vec<Vec<f32>> = vec![Vec::new(); 4];
    let dists = [0.3_f32, 1.0, 2.0, 3.0];
    for (n, a, b, c) in count.values() {
        if *n != 1 {
            continue;
        }
        let ab = (*b - *a).truncate();
        let len = ab.length();
        if len < 0.5 {
            continue;
        }
        let dir = ab / len;
        let mut normal = Vec2::new(-dir.y, dir.x);
        if normal.dot((*c - *a).truncate()) > 0.0 {
            normal = -normal;
        }
        let mut s = 0.5;
        while s < len {
            let p = a.truncate() + dir * s;
            let h = a.z + (b.z - a.z) * (s / len);
            for (i, d) in dists.iter().enumerate() {
                let q = p + normal * *d;
                if (1.0..299.0).contains(&q.x) && (1.0..299.0).contains(&q.y) {
                    gaps[i].push(carved.height(q.x, q.y) - h);
                }
            }
            s += 1.0;
        }
    }
    for (i, d) in dists.iter().enumerate() {
        // Gaps of more than 0.75 m are content (embankments, cuttings, bridges): the fit leaves
        // them, so they are not counted here.
        let g = &mut gaps[i];
        g.retain(|x| x.abs() <= 0.75);
        if g.is_empty() {
            continue;
        }
        g.sort_by(f32::total_cmp);
        let pct = |p: f32| g[((g.len() - 1) as f32 * p) as usize];
        let over =
            |t: f32| g.iter().filter(|x| x.abs() > t).count() as f32 / g.len() as f32 * 100.0;
        if i == 0 {
            assert!(
                pct(0.5).abs() < 0.05 && over(0.20) < 12.0,
                "the terrain 0.3 m outside the edges is off by {:+.3} m (median), {:.1} % by more than 20 cm",
                pct(0.5),
                over(0.20)
            );
        }
        eprintln!(
            "{d:.1} m outside: n={} p05 {:+.3} p50 {:+.3} p95 {:+.3}; |gap|>5cm {:.1} %, >20cm {:.1} %, >50cm {:.1} %",
            g.len(),
            pct(0.05),
            pct(0.5),
            pct(0.95),
            over(0.05),
            over(0.20),
            over(0.50)
        );
    }
}
