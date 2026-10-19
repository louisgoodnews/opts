//! Can every `[attachObj]` of the installed maps be placed? For each record: is its parent on the
//! same tile (an `[object]`, a spline attachment or another `[attachObj]`), and does the parent's
//! `.sco` have the attachment point it names (`[new_attachment]`, file order)?
//!
//! Usage: `attach_obj_census <omsi root> [map folder]` (all maps when no folder is given)
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use opts_omsi::formats::map::MapTile;
use opts_omsi::formats::scenery::sco::SceneryObject;

/// Case-insensitive, separator-agnostic path lookup below `base`.
fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let direct = cur.join(part);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let want = part.to_lowercase();
        cur = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().to_lowercase() == want)?
            .path();
    }
    Some(cur)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("omsi root"));
    let only = args.next();
    let maps_dir = root.join("maps");
    let mut maps: Vec<PathBuf> = std::fs::read_dir(&maps_dir)
        .expect("maps")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            only.as_ref()
                .is_none_or(|o| p.file_name().is_some_and(|n| n.to_string_lossy() == *o))
        })
        .collect();
    maps.sort();
    let mut points: HashMap<String, Option<usize>> = HashMap::new();
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    let mut examples: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for map in &maps {
        for e in std::fs::read_dir(map).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !(name.starts_with("tile_") && name.ends_with(".map")) {
                continue;
            }
            let Ok(tile) = std::fs::read(e.path())
                .map_err(|e| e.to_string())
                .and_then(|b| MapTile::parse(&b).map_err(|e| e.to_string()))
            else {
                continue;
            };
            let mut sco_of: HashMap<u64, &str> = HashMap::new();
            for o in &tile.objects {
                sco_of.entry(o.id_code).or_insert(&o.sco_file);
            }
            for a in &tile.spline_attachments {
                sco_of.entry(a.id_code).or_insert(&a.sco_file);
            }
            for a in &tile.attached_objects {
                sco_of.entry(a.id_code).or_insert(&a.sco_file);
            }
            for a in &tile.attached_objects {
                let verdict = match sco_of.get(&a.parent_id) {
                    None => "parent not on the tile",
                    Some(parent) => {
                        let n = *points.entry(parent.to_lowercase()).or_insert_with(|| {
                            let p = find_ci(&root, parent)?;
                            let b = std::fs::read(p).ok()?;
                            Some(SceneryObject::parse(&b).ok()?.attachments.len())
                        });
                        match n {
                            None => "parent .sco missing or unreadable",
                            Some(n) if a.attachment < n => "placeable",
                            Some(_) => "parent lacks the attachment point",
                        }
                    }
                };
                *tally.entry(verdict).or_default() += 1;
                let ex = examples.entry(verdict).or_default();
                if ex.len() < 4 && verdict != "placeable" {
                    ex.push(format!(
                        "{} {name}: {} on {} point {}",
                        map.file_name().unwrap_or_default().to_string_lossy(),
                        a.sco_file,
                        sco_of.get(&a.parent_id).copied().unwrap_or("?"),
                        a.attachment
                    ));
                }
            }
        }
    }
    for (k, n) in &tally {
        println!("{n:7}  {k}");
        for e in examples.get(k).into_iter().flatten() {
            println!("           {e}");
        }
    }
}
