//! What do the two `priority` values (`192`, `64`) of `[rule]` mean? Classifies the junction paths
//! they address by turn direction. Usage: `-- <OMSI root>`.
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use opts_core::formats::map::{MapTile, RuleKind, RuleOwner};
use opts_core::formats::scenery::SceneryObject;

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
    let mut cache: HashMap<String, Option<SceneryObject>> = HashMap::new();
    let mut stats: BTreeMap<(u32, &str), usize> = BTreeMap::new();
    let mut per_object: BTreeMap<(usize, usize), usize> = BTreeMap::new(); // (n192, n64) per rule set
    let mut with_light = BTreeMap::new();
    for m in std::fs::read_dir(root.join("maps")).unwrap().flatten() {
        let Ok(rd) = std::fs::read_dir(m.path()) else { continue };
        for f in rd.flatten() {
            let n = f.file_name().to_string_lossy().to_lowercase();
            if !(n.starts_with("tile_") && n.ends_with(".map")) {
                continue;
            }
            let Some(tile) = std::fs::read(f.path()).ok().and_then(|b| MapTile::parse(&b).ok()) else { continue };
            let mut seen_obj: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
            for r in tile.rules.iter().filter(|r| r.kind() == Some(RuleKind::Priority)) {
                let Some(RuleOwner::Object { index, .. }) = r.owner else { continue };
                let file = tile.objects[index].sco_file.clone();
                let sco = cache.entry(file.to_ascii_lowercase()).or_insert_with(|| {
                    SceneryObject::parse(&std::fs::read(find_ci(&root, &file)?).ok()?).ok()
                });
                let Some(sco) = sco else { continue };
                let Some(p) = sco.paths.get(r.path_index) else { continue };
                let v = r.value as u32;
                let class = if p.radius_m.abs() < 1.0 || p.radius_m.abs() > 900.0 {
                    "straight"
                } else {
                    let ang = (p.length_m / p.radius_m).to_degrees();
                    if ang.abs() < 20.0 { "straight" } else if ang > 0.0 { "right" } else { "left" }
                };
                *stats.entry((v, class)).or_default() += 1;
                *with_light.entry((v, p.traffic_light_index.is_some())).or_insert(0usize) += 1;
                let e = seen_obj.entry(index).or_default();
                if v == 192 { e.0 += 1 } else { e.1 += 1 }
            }
            for (_, (a, b)) in seen_obj {
                *per_object.entry((a.min(9), b.min(9))).or_default() += 1;
            }
        }
    }
    println!("(value, turn) -> paths: {stats:?}");
    println!("(value, governed by a light) -> paths: {with_light:?}");
    println!("objects by (#paths with 192, #paths with 64), capped at 9: {:?}", per_object.iter().take(12).collect::<Vec<_>>());
}
