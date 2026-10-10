//! Census of every spline placed on a map: which profiles resolve, draw, and find their textures.
//! Usage: `cargo run --release -p opts-bevy --example spline_census -- <map dir>`.
use std::collections::BTreeMap;

use opts_bevy::install::OmsiInstall;
use opts_bevy::scenery::extrude_spline_segment;
use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::sli::SplineProfile;

#[derive(Default)]
struct Row {
    count: usize,
    problem: String,
}

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let root = OmsiInstall::root_from_map_dir(&dir).expect("OMSI root");
    let install = OmsiInstall::new(&root);
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for e in std::fs::read_dir(&dir).expect("dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("tile_") && name.ends_with(".map")) {
            continue;
        }
        let Ok(tile) = MapTile::parse(&std::fs::read(e.path()).expect("tile")) else {
            println!("tile parse failed: {name}");
            continue;
        };
        for s in &tile.splines {
            let row = rows.entry(s.sli_file.clone()).or_default();
            row.count += 1;
            if !row.problem.is_empty() {
                continue;
            }
            let Some(path) = install.find_file(&s.sli_file) else {
                row.problem = "sli not found".into();
                continue;
            };
            let profile = match std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|b| SplineProfile::parse(&b).map_err(|e| e.to_string()))
            {
                Ok(p) => p,
                Err(e) => {
                    row.problem = format!("parse: {e}");
                    continue;
                }
            };
            if profile.only_editor {
                row.problem = "only_editor".into();
                continue;
            }
            if extrude_spline_segment(&profile, s, (0.0, 0.0)).is_empty() {
                row.problem = format!(
                    "no geometry (graphical {}, paths {})",
                    profile.graphical_profiles.len(),
                    profile.paths.len()
                );
                continue;
            }
            let dir = path.parent().unwrap().to_path_buf();
            let missing: Vec<&str> = profile
                .textures
                .iter()
                .filter(|t| {
                    let n = t.file.replace('\\', "/");
                    ![
                        dir.join("texture"),
                        dir.clone(),
                        dir.parent().unwrap().join("texture"),
                        dir.join("Texture"),
                        root.join("Texture"),
                        dir.join(".."),
                    ]
                    .iter()
                    .any(|d| {
                        install.find_in(d, &n).is_some()
                            || install.find_in(d, n.rsplit('/').next().unwrap()).is_some()
                    })
                })
                .map(|t| t.file.as_str())
                .collect();
            if !missing.is_empty() {
                row.problem = format!("texture missing: {missing:?}");
            }
        }
    }
    let total: usize = rows.values().map(|r| r.count).sum();
    let bad: usize = rows
        .values()
        .filter(|r| !r.problem.is_empty())
        .map(|r| r.count)
        .sum();
    println!(
        "{total} splines, {} profiles, {bad} splines with problems",
        rows.len()
    );
    for (k, r) in rows.iter().filter(|(_, r)| !r.problem.is_empty()) {
        println!("{:>5}  {k}  -> {}", r.count, r.problem);
    }
}
