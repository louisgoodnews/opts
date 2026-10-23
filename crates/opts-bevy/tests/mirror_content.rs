//! Mirror panes on real vehicles: the identification the mirror cameras rely on
//! ([`opts_bevy::glass::classify`], pane texture `reflexion<N>.bmp`) applied to the first LOD of a
//! vehicle, looking at the textures inside the `.o3d` files and at the faces that really use them
//! (the loader makes a part, and so a mirror pane, only for a material with faces). Skipped
//! without `OMSI_PATH`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opts_bevy::glass::{classify, SurfaceKind};
use opts_core::formats::vehicles::model::ModelConfig;

fn root() -> Option<PathBuf> {
    let root = std::env::var_os("OMSI_PATH").map(PathBuf::from);
    if root.is_none() {
        eprintln!("OMSI_PATH not set; skipping");
    }
    root
}

/// Pane textures of the first LOD with the number of mesh files that have faces on them, and
/// the number of meshes of that LOD.
fn panes(vehicle_root: &Path, cfg: &Path) -> Option<(BTreeMap<String, usize>, usize)> {
    let bytes = std::fs::read(cfg).ok()?;
    let model = ModelConfig::parse_bytes(&bytes).ok()?;
    let lod0 = model.lods.first()?;
    let mut found: BTreeMap<String, usize> = BTreeMap::new();
    for mesh in &lod0.meshes {
        let rel = mesh.path.replace('\\', "/");
        let Ok(files) = opts_bevy::load_decoded_meshes(&vehicle_root.join(&rel)) else {
            continue;
        };
        for o in &files {
            let mut used: Vec<String> = Vec::new();
            for slot in o.face_materials.iter().copied() {
                let Some(tex) = o
                    .materials
                    .get(slot as usize)
                    .and_then(|m| m.primary_texture())
                else {
                    continue;
                };
                if classify(tex, false) == Some(SurfaceKind::Mirror) {
                    let tex = tex.to_lowercase();
                    if !used.contains(&tex) {
                        used.push(tex);
                    }
                }
            }
            for t in used {
                *found.entry(t).or_default() += 1;
            }
        }
    }
    Some((found, lod0.meshes.len()))
}

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

/// The whole chain on real meshes: the loader registers the pane materials, and the mirror
/// system turns the panes of a player vehicle into cameras (one per `reflexion<N>` material).
#[test]
fn loaded_mirror_parts_become_mirror_cameras_on_the_player_vehicle_only() {
    use bevy::prelude::*;
    use opts_bevy::mirrors::{
        spawn_mirror_cameras, update_mirror_cameras, MirrorCamera, MirrorFrame, MirrorSettings,
        MirrorStats,
    };
    use opts_bevy::{ContentCache, ContentLoader, OmsiInstall, PlayerVehicle};

    let Some(root) = root() else { return };
    let dir = root.join("Vehicles/MAN_NewLionsCity/model/generic");
    if !dir.join("Spiegel_links.o3d").exists() {
        eprintln!("MAN_NewLionsCity not installed; skipping");
        return;
    }
    let install = OmsiInstall::new(&root);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<Mesh>>()
        .insert_resource(opts_bevy::light_quality::LightQuality::default())
        .insert_resource(MirrorSettings {
            scan_interval_s: 0.0,
            ..default()
        })
        .init_resource::<MirrorFrame>()
        .init_resource::<MirrorStats>()
        .add_systems(Update, spawn_mirror_cameras)
        .add_systems(PostUpdate, update_mirror_cameras);
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        GlobalTransform::default(),
    ));

    let mut cache = ContentCache::default();
    let mut parts = Vec::new();
    {
        let world = app.world_mut();
        let mut meshes = world.remove_resource::<Assets<Mesh>>().unwrap();
        let mut materials = world.remove_resource::<Assets<StandardMaterial>>().unwrap();
        let mut images = world.remove_resource::<Assets<Image>>().unwrap();
        for file in [
            "Spiegel_links.o3d",
            "Spiegel_rechts_II.o3d",
            "Innenspiegel.o3d",
        ] {
            let mut loader = ContentLoader {
                install: &install,
                cache: &mut cache,
                meshes: &mut meshes,
                materials: &mut materials,
                images: &mut images,
            };
            parts.extend(loader.o3d_parts(&dir.join(file), &[], &[]));
        }
        world.insert_resource(meshes);
        world.insert_resource(materials);
        world.insert_resource(images);
    }
    let panes = parts
        .iter()
        .filter(|p| opts_bevy::glass::surface_kind(p.material.id()) == Some(SurfaceKind::Mirror))
        .count();
    assert!(
        panes >= 3,
        "pane parts registered by the loader: {panes} of {}",
        parts.len()
    );

    let spawn_vehicle = |app: &mut App, player: bool| {
        let root = app.world_mut().spawn(Transform::default()).id();
        if player {
            app.world_mut().entity_mut(root).insert(PlayerVehicle);
        }
        let mut cmds = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            cmds.push(
                app.world_mut()
                    .spawn((
                        Mesh3d(part.mesh.clone()),
                        MeshMaterial3d(part.material.clone()),
                        Transform::from_xyz(i as f32 * 0.5, 0.0, -3.0),
                        GlobalTransform::from_xyz(i as f32 * 0.5, 0.0, -3.0),
                    ))
                    .id(),
            );
        }
        for c in cmds {
            app.world_mut().entity_mut(root).add_child(c);
        }
    };
    spawn_vehicle(&mut app, false);
    app.update();
    let mut q = app.world_mut().query::<&MirrorCamera>();
    assert_eq!(
        q.iter(app.world()).count(),
        0,
        "no player vehicle, no mirror cameras"
    );

    spawn_vehicle(&mut app, true);
    app.update();
    app.update();
    let mut q = app.world_mut().query::<&MirrorCamera>();
    let groups: Vec<_> = q.iter(app.world()).map(|c| c.members.len()).collect();
    eprintln!("mirror groups (panes each): {groups:?}");
    assert!(
        groups.len() >= 3,
        "reflexion0, 1 and 3 at least: {groups:?}"
    );
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
