//! Per-map census of scenery objects: how many placed instances end up with drawable geometry
//! and why the rest does not (missing `.sco`, missing mesh, unparseable, unreadable layout,
//! extent gates). Keyed meshes are decoded exactly (`ok:exact`).
//!
//! Usage: `map_object_census <omsi root> <map folder name> [--list]`
//!
//! `.x` files are parsed too (`fail:x-parse`, `none:x-empty`). `MESH_LIST=<file>` writes one line
//! per drawn mesh: path, verdict, instances, the vertex layout the renderer uses, the floor axes of
//! the drawn positions and an orientation verdict (`upright`, `lying` when the mesh stands on an
//! axis the renderer does not turn up, `no-floor` when no axis is a floor).
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_core::formats::map::MapTile;
use opts_core::formats::o3d::{parse, VertexLayout};
use opts_core::formats::scenery::sco::SceneryObject;
use opts_core::formats::x::parse_x;

/// Case-insensitive, separator-agnostic path lookup below `base`.
fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    // The base itself may differ in case (`Model` for `model`).
    let mut cur = if base.exists() {
        base.to_path_buf()
    } else {
        let parent = base.parent()?;
        let want = base.file_name()?.to_string_lossy().to_lowercase();
        std::fs::read_dir(parent)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().to_lowercase() == want)?
            .path()
    };
    for part in rel
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
    {
        if part == ".." {
            cur = cur.parent()?.to_path_buf();
            continue;
        }
        let direct = cur.join(part);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let want = part.to_lowercase();
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().to_lowercase() == want)?;
        cur = hit.path();
    }
    Some(cur)
}

fn extent(o: &opts_core::formats::o3d::O3d) -> f32 {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &o.vertices {
        let p = [v.position.x, v.position.y, v.position.z];
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (0..3).map(|k| hi[k] - lo[k]).fold(0.0, f32::max)
}

/// Triangles of every sub-mesh of an `.x` file (frames included).
fn x_triangles(x: &opts_core::formats::x::XMesh) -> usize {
    fn frame(f: &opts_core::formats::x::XFrame) -> usize {
        f.meshes.iter().map(|m| m.indices.len()).sum::<usize>()
            + f.children.iter().map(frame).sum::<usize>()
    }
    x.top_level_meshes
        .iter()
        .map(|m| m.indices.len())
        .sum::<usize>()
        + x.frames.iter().map(frame).sum::<usize>()
}

/// Layout, floor axes and orientation verdict of a drawn `.o3d` (see `MESH_LIST`).
fn orientation(o: &opts_core::formats::o3d::O3d) -> String {
    let layout = o.vertex_layout();
    let floor = o.floor_axes();
    // The renderer turns stored `x` up for axis-cycled meshes, stored `y` otherwise.
    let up = if layout == VertexLayout::AxisCycled {
        0
    } else {
        1
    };
    let verdict = match floor.as_slice() {
        [] => "no-floor",
        f if f.contains(&up) => "upright",
        _ => "lying",
    };
    format!("{layout:?}\t{floor:?}\t{verdict}")
}

thread_local! {
    static ORIENT: std::cell::RefCell<HashMap<PathBuf, String>> = std::cell::RefCell::new(HashMap::new());
}

/// The verdict for one mesh file.
fn classify(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return "fail:unreadable-file".into();
    };
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("x"))
    {
        return match parse_x(&bytes) {
            Ok(x) if x_triangles(&x) == 0 => "none:x-empty".into(),
            Ok(_) => "ok:x-file".into(),
            Err(e) => format!(
                "fail:x-parse({})",
                e.to_string().chars().take(40).collect::<String>()
            ),
        };
    }
    let o = match parse(&bytes) {
        Ok(o) => o,
        Err(e) => {
            return format!(
                "fail:parse({})",
                e.to_string().chars().take(40).collect::<String>()
            )
        }
    };
    if o.vertices.is_empty() || o.indices.is_empty() {
        return "none:empty-mesh".into();
    }
    if o.encryption_key.is_some() {
        let mut exact = o.clone();
        if opts_core::formats::o3d::exact::decode(&mut exact).is_some() {
            ORIENT.with(|m| {
                m.borrow_mut()
                    .insert(path.to_path_buf(), orientation(&exact))
            });
            return if extent(&exact) > 400.0 {
                "fail:exact-extent>400".into()
            } else {
                "ok:exact".into()
            };
        }
    }
    ORIENT.with(|m| m.borrow_mut().insert(path.to_path_buf(), orientation(&o)));
    let ext = extent(&o);
    if ext > 3000.0 {
        "fail:plain-extent>3000".into()
    } else {
        "ok:plain".into()
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let root = PathBuf::from(a.next().expect("omsi root"));
    let map = a.next().expect("map folder");
    let list = a.next().as_deref() == Some("--list");
    let dir = root.join("maps").join(&map);
    let mut tiles = 0;
    let mut tile_fail = 0;
    let mut inst: HashMap<String, usize> = HashMap::new();
    for e in std::fs::read_dir(&dir).expect("map dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("tile_") && name.ends_with(".map")) {
            continue;
        }
        tiles += 1;
        match std::fs::read(e.path())
            .map_err(|e| e.to_string())
            .and_then(|b| MapTile::parse(&b).map_err(|e| e.to_string()))
        {
            Ok(t) => {
                for o in t.objects {
                    *inst.entry(o.sco_file).or_default() += 1;
                }
            }
            Err(err) => {
                tile_fail += 1;
                println!("TILE_FAIL {name}: {err}");
            }
        }
    }
    println!(
        "tiles {tiles} (parse failures {tile_fail}); distinct sco {}; instances {}",
        inst.len(),
        inst.values().sum::<usize>()
    );
    let mut by_class: HashMap<String, (usize, usize, Vec<String>)> = HashMap::new();
    let mut cache: HashMap<PathBuf, String> = HashMap::new();
    let mut mesh_inst: HashMap<PathBuf, usize> = HashMap::new();
    for (sco_rel, n) in &inst {
        let class = (|| {
            let Some(sco_path) = find_ci(&root, sco_rel) else {
                return "fail:sco-missing".to_string();
            };
            let Ok(bytes) = std::fs::read(&sco_path) else {
                return "fail:sco-unreadable".into();
            };
            let sco = match SceneryObject::parse(&bytes) {
                Ok(s) => s,
                Err(_) => return "fail:sco-parse".into(),
            };
            if sco.model_cfg.is_some() {
                return "ok:vehicle-model".into();
            }
            if sco.meshes.is_empty() {
                return "none:no-mesh(helper)".into();
            }
            let model_dir = sco_path
                .parent()
                .map(|p| p.join("model"))
                .unwrap_or_default();
            let sco_dir = sco_path.parent().unwrap_or(Path::new("."));
            let mut worst = String::from("ok");
            let mut verdicts = Vec::new();
            for m in &sco.meshes {
                let Some(p) = find_ci(&model_dir, &m.file)
                    .filter(|p| p.is_file())
                    .or_else(|| find_ci(sco_dir, &m.file).filter(|p| p.is_file()))
                else {
                    verdicts.push("fail:o3d-not-found".to_string());
                    continue;
                };
                *mesh_inst.entry(p.clone()).or_default() += n;
                let v = cache
                    .entry(p.clone())
                    .or_insert_with(|| classify(&p))
                    .clone();
                verdicts.push(v);
            }
            for v in &verdicts {
                let rank = |s: &str| match s.split(':').next() {
                    Some("fail") => 3,
                    Some("none") => 1,
                    _ => 0,
                };
                if rank(v) > rank(&worst) {
                    worst = v.clone();
                }
            }
            if worst == "ok" {
                verdicts.first().cloned().unwrap_or("ok".into())
            } else {
                worst
            }
        })();
        let e = by_class.entry(class).or_default();
        e.0 += 1;
        e.1 += n;
        e.2.push(format!("{n}× {sco_rel}"));
    }
    if let Ok(out) = std::env::var("FAIL_LIST") {
        let mut s = String::new();
        for (p, v) in &cache {
            if v.starts_with("fail:") {
                s.push_str(&format!("{}\t{v}\n", p.display()));
            }
        }
        std::fs::write(out, s).unwrap();
    }
    if let Ok(out) = std::env::var("MESH_LIST") {
        let mut s = String::new();
        ORIENT.with(|m| {
            for (p, o) in m.borrow().iter() {
                s.push_str(&format!(
                    "{}\t{}\t{}\t{o}\n",
                    p.display(),
                    cache.get(p).map_or("", String::as_str),
                    mesh_inst.get(p).copied().unwrap_or(0)
                ));
            }
        });
        std::fs::write(out, s).unwrap();
    }
    let mut rows: Vec<_> = by_class.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1 .1));
    for (class, (objs, instances, names)) in rows {
        println!("{class:50} sco={objs:5} instances={instances:6}");
        if list || !class.starts_with("ok") {
            let mut names = names;
            names.sort();
            for n in names.iter().take(if list { 1000 } else { 12 }) {
                println!("      {n}");
            }
        }
    }
}
