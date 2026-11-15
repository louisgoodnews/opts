//! The mirror panes of the vehicles of the install: the first LOD's pane textures, the reference bus and the budget.

use super::*;

#[test]
fn a_bus_with_mirror_panes_has_several() {
    let Some(root) = root() else { return };
    let dir = root.join("Vehicles/MAN_NewLionsCity/model");
    let cfg = dir.join("model_19C_4door_main.cfg");
    if !cfg.exists() {
        eprintln!("MAN_NewLionsCity not installed; skipping");
        return;
    }
    let (found, total) = panes(&dir, &cfg).expect("model cfg");
    eprintln!("{total} meshes; pane textures (mesh files with faces on them): {found:?}");
    assert!(
        found.len() >= 3,
        "side mirrors and an interior mirror, got {found:?}"
    );
    assert!(
        found.values().sum::<usize>() >= 5,
        "variants of the mirrors included: {found:?}"
    );
}

#[test]
fn the_reference_bus_has_three_mirror_panes() {
    let Some(root) = root() else { return };
    let dir = root.join("Vehicles/MAN_SD200/Model");
    let cfg = dir.join("model_SD77.cfg");
    if !cfg.exists() {
        eprintln!("MAN_SD200 not installed; skipping");
        return;
    }
    let (found, total) = panes(&dir, &cfg).expect("model cfg");
    eprintln!("SD77: {total} meshes, panes {found:?}");
    assert!(total > 0);
    assert_eq!(found.len(), 3, "reflexion0 to reflexion2: {found:?}");
}

#[test]
fn a_vehicle_without_mirrors_has_none() {
    let Some(root) = root() else { return };
    let dir = root.join("Vehicles/Bad_Huegelsdorf_Schiff/model");
    let cfg = dir.join("model.cfg");
    if !cfg.exists() {
        eprintln!("Bad_Huegelsdorf_Schiff not installed; skipping");
        return;
    }
    let (found, total) = panes(&dir, &cfg).expect("model cfg");
    eprintln!("ship: {total} meshes, panes {found:?}");
    assert!(total > 0);
    assert!(found.is_empty(), "no mirror pane on a ship: {found:?}");
}

/// Every vehicle model of the install: how many mirror numbers (cameras) its first LOD has, and
/// that the camera budget covers them.
#[test]
fn no_vehicle_needs_more_mirror_cameras_than_exist() {
    let Some(root) = root() else { return };
    let vehicles = root.join("Vehicles");
    let Ok(dirs) = std::fs::read_dir(&vehicles) else {
        return;
    };
    let mut table: Vec<(usize, String, Vec<String>)> = Vec::new();
    for dir in dirs.flatten() {
        let model = dir.path().join("model");
        let Ok(cfgs) = std::fs::read_dir(&model) else {
            continue;
        };
        for cfg in cfgs.flatten() {
            let path = cfg.path();
            if path
                .extension()
                .is_none_or(|e| !e.eq_ignore_ascii_case("cfg"))
            {
                continue;
            }
            let Some((found, _)) = panes(&model, &path) else {
                continue;
            };
            if found.is_empty() {
                continue;
            }
            let name = format!(
                "{}/{}",
                dir.file_name().to_string_lossy(),
                path.file_name().unwrap().to_string_lossy()
            );
            table.push((found.len(), name, found.keys().cloned().collect()));
        }
    }
    table.sort();
    for (n, name, textures) in table.iter().rev().take(25) {
        eprintln!("{n:2} mirror numbers  {name}  {textures:?}");
    }
    eprintln!("{} models with mirror panes", table.len());
    let over: Vec<_> = table
        .iter()
        .filter(|(n, ..)| *n > opts_bevy::mirrors::MAX_CAMERAS)
        .collect();
    assert!(
        over.is_empty(),
        "more mirror numbers than cameras: {over:?}"
    );
}
