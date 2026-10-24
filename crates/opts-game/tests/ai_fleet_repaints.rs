//! Content check of the depot fleet repaints (`ailists.cfg`): how many of the repaint names a
//! map's fleet rows give are repaints the vehicle has. Skipped unless `OMSI_PATH` points at an
//! installation with Winsenburg. Prints the names that do not resolve.

use std::collections::BTreeSet;
use std::path::PathBuf;

use opts_core::formats::map::AiLists;
use opts_core::formats::vehicles::BusManifest;

#[test]
fn fleet_repaints_resolve_on_their_vehicles() {
    for map in ["Berlin-Spandau", "Winsenburg", "Ahlheim 5", "Bremen-Nord"] {
        check(map);
    }
}

fn check(map: &str) {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let install = opts_bevy::install::OmsiInstall::new(&root);
    let Some(dir) = install.find_in(&root.join("maps"), map) else {
        eprintln!("{map} not installed; skipping");
        return;
    };
    let lists = AiLists::parse(&std::fs::read(dir.join("ailists.cfg")).expect("ailists"));
    let (mut rows, mut found) = (0, 0);
    let mut missing: BTreeSet<(String, String)> = BTreeSet::new();
    for g in &lists.depot_groups {
        for (file, e) in &g.fleet {
            if e.repaint.is_empty() {
                continue;
            }
            let Some(path) = install.find_file(file) else {
                continue;
            };
            let Ok(manifest) = BusManifest::parse_bytes(&std::fs::read(&path).unwrap_or_default())
            else {
                continue;
            };
            let Some(cfg_rel) = manifest.model_cfg.clone() else {
                continue;
            };
            let vdir = path.parent().unwrap().to_path_buf();
            let Some(cfg) = install.find_in(&vdir, &cfg_rel) else {
                continue;
            };
            let Ok(model) = opts_core::formats::vehicles::ModelConfig::parse_bytes(
                &std::fs::read(cfg).unwrap_or_default(),
            ) else {
                continue;
            };
            let default = manifest
                .friendly_name
                .as_ref()
                .map(|f| f.livery.clone())
                .unwrap_or_default();
            let repaints = opts_game::new_game_screen::repaints::discover_for_model(
                &root, &vdir, &model, &default,
            );
            rows += 1;
            if repaints.named(&e.repaint).is_some() {
                found += 1;
            } else {
                missing.insert((file.clone(), e.repaint.clone()));
            }
        }
    }
    eprintln!("{map}: {found} of {rows} fleet rows name a repaint their vehicle has");
    // Winsenburg keeps its `texture` and `Texture` folders side by side: all 117 rows resolve.
    if map == "Winsenburg" {
        assert_eq!(found, rows, "Winsenburg: {found} of {rows}");
    }
    for (f, r) in missing.iter().take(8) {
        eprintln!("  not found: {r} on {f}");
    }
}
