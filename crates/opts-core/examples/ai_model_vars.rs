//! Which script variables do the models of AI vehicles use? Reads every vehicle named in an
//! `ailists*.cfg` of the installation and counts the variables of `[newanim]`, `[visible]`,
//! `[matl_lightmap]` and `[interiorlight]`. Usage: `-- <OMSI root>`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use opts_core::formats::map::AiLists;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
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

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("OMSI root"));
    let mut vehicles = BTreeSet::new();
    for m in std::fs::read_dir(root.join("maps")).unwrap().flatten() {
        for f in std::fs::read_dir(m.path()).into_iter().flatten().flatten() {
            let n = f.file_name().to_string_lossy().to_lowercase();
            if n.starts_with("ailists") && n.ends_with(".cfg") {
                if let Ok(b) = std::fs::read(f.path()) {
                    for g in AiLists::parse(&b).groups {
                        for e in g.entries {
                            vehicles.insert(e.vehicle.to_lowercase());
                        }
                    }
                }
            }
        }
    }
    let mut anim: BTreeMap<String, usize> = BTreeMap::new();
    let mut vis: BTreeMap<String, usize> = BTreeMap::new();
    let mut light: BTreeMap<String, usize> = BTreeMap::new();
    let mut inter: BTreeMap<String, usize> = BTreeMap::new();
    let (mut ok, mut missing, mut spots) = (0, 0, 0usize);
    for v in &vehicles {
        let Some(path) = find_ci(&root, v) else {
            missing += 1;
            continue;
        };
        let dir = path.parent().unwrap().to_path_buf();
        let Some(manifest) = std::fs::read(&path)
            .ok()
            .and_then(|b| BusManifest::parse_bytes(&b).ok())
        else {
            continue;
        };
        let Some(cfg) = manifest.model_cfg.as_deref().and_then(|c| find_ci(&dir, c)) else {
            continue;
        };
        let Some(model) = std::fs::read(&cfg)
            .ok()
            .and_then(|b| ModelConfig::parse_bytes(&b).ok())
        else {
            continue;
        };
        ok += 1;
        let Some(lod) = model
            .lods
            .iter()
            .find(|l| l.threshold == 0.0)
            .or(model.lods.first())
        else {
            continue;
        };
        for m in &lod.meshes {
            for a in &m.anims {
                for x in a.anim_rot.iter().chain(a.anim_trans.iter()) {
                    *anim.entry(x.variable.to_lowercase()).or_default() += 1;
                }
            }
            for x in &m.visibility {
                *vis.entry(x.variable.to_lowercase()).or_default() += 1;
            }
            for mat in &m.materials {
                if let Some(l) = &mat.light_map {
                    *light.entry(l.variable.to_lowercase()).or_default() += 1;
                }
            }
            for i in &m.interior_lights {
                *inter.entry(i.variable.to_lowercase()).or_default() += 1;
            }
            spots += m.spotlights.len();
        }
        spots += model.spotlights.len();
    }
    println!(
        "{} vehicles listed, {ok} models read, {missing} files missing, {spots} spotlights",
        vehicles.len()
    );
    for (name, map) in [
        ("anim", anim),
        ("visible", vis),
        ("lightmap", light),
        ("interiorlight", inter),
    ] {
        let mut v: Vec<_> = map.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        println!("{name}: {:?}", &v[..v.len().min(24)]);
    }
}
