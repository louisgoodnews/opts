//! Unit tests of `lib.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use std::fs;

fn fixture() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("opts_cat_{}_{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Vehicles/Bus1/model")).unwrap();
    fs::create_dir_all(dir.join("maps/Town")).unwrap();
    fs::write(
        dir.join("Vehicles/Bus1/Bus1.bus"),
        "[friendlyname]\nACME\nCity 1\nRed\n\n[model]\nmodel\\model.cfg\n\n\
             [passengercabin]\nmodel\\cabin.cfg\n\n\
             [add_camera_driver]\n0.5\n0.3\n1.2\n0\n60\n0\n0\n",
    )
    .unwrap();
    fs::write(
        dir.join("Vehicles/Bus1/model/cabin.cfg"),
        "[drivpos]\n0.5\n0.3\n1.0\n0\n",
    )
    .unwrap();
    fs::write(
        dir.join("Vehicles/Bus1/Bus1_trail.bus"),
        "[friendlyname]\nACME\nTrailer\nRed\n",
    )
    .unwrap();
    fs::write(dir.join("Vehicles/Bus1/model/model.cfg"), "x").unwrap();
    fs::write(dir.join("maps/Town/global.cfg"), "[name]\nTown\n").unwrap();
    dir
}

#[test]
fn discovers_entities_and_resolves_paths_case_insensitively() {
    let root = fixture();
    fs::create_dir_all(root.join("Vehicles/Rail")).unwrap();
    fs::write(
        root.join("Vehicles/Rail/Coach.ovh"),
        "[type]\n2\n\n[model]\nmodel\\model.cfg\n\n[boogies]\n9.5\n\n[mass]\n40\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("Trains")).unwrap();
    fs::write(root.join("Trains/b.zug"), "a.ovh\r\n0\r\nb.ovh\r\n1").unwrap();
    fs::write(root.join("Trains/A.zug"), "a.ovh\r\n0\r\nb.ovh\r\n0").unwrap();
    let db = root.join("cat.db");
    let summary = discover(&db, &root, &|_| {}).unwrap();
    assert!(summary.files >= 4 && summary.entities >= 3);

    let cat = Catalogue::open(&db).unwrap();
    let maps = cat.maps().unwrap();
    assert_eq!(maps.len(), 1);
    assert_eq!(maps[0].folder, "Town");

    let vehicles = cat.playable_vehicles().unwrap();
    assert_eq!(
        vehicles.len(),
        1,
        "the *_trail.bus segment is not player-selectable"
    );
    assert_eq!(vehicles[0].id, "Bus1");
    assert_eq!(vehicles[0].manufacturer, "ACME");

    // Rail cars: every rail file, whether a player may select it or not.
    let rail = cat.rail_vehicles().unwrap();
    assert_eq!(rail.len(), 1, "{rail:?}");
    assert_eq!(rail[0].id, "Coach");
    assert_eq!(rail[0].marker_file, "Vehicles/Rail/Coach.ovh");
    assert_eq!(rail[0].mass_tonnes, Some(40.0));
    assert_eq!(rail[0].class.as_deref(), Some("rail"));
    assert!(
        !vehicles.iter().any(|v| v.id == "Coach"),
        "an .ovh is never player-selectable"
    );

    assert_eq!(
        cat.train_consists().unwrap(),
        vec!["Trains/A.zug".to_string(), "Trains/b.zug".to_string()],
        "consists by path, case-insensitively sorted"
    );

    let ro = Catalogue::open_read_only(&db).unwrap();
    assert_eq!(ro.maps().unwrap().len(), 1);
    assert!(Catalogue::open_read_only(&root.join("missing.db")).is_err());

    let found = cat.find_file("vehicles\\BUS1\\MODEL\\Model.CFG").unwrap();
    assert!(found.is_some_and(|p| p.ends_with("Vehicles/Bus1/model/model.cfg")));
    assert!(cat
        .find_file("Vehicles/Bus1/missing.cfg")
        .unwrap()
        .is_none());
    assert_eq!(
        cat.vehicle_marker("bus1").unwrap().as_deref(),
        Some("Vehicles/Bus1/Bus1.bus")
    );
    let counts = cat.kind_counts().unwrap();
    assert_eq!(
        counts.maps,
        maps.len() as i64,
        "same number as the maps list"
    );
    assert_eq!(
        counts.vehicles,
        vehicles.len() as i64,
        "same number as the vehicle list"
    );
    assert!(counts.entities >= counts.maps + counts.vehicles);
    assert_eq!(
        counts.maps + counts.vehicles + counts.scenery_objects + counts.splines + counts.other,
        counts.entities,
        "the kinds add up to the entries"
    );
    assert!(counts.files >= 4);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn staleness_tracks_age_and_root() {
    let root = fixture();
    let db = root.join("cat.db");
    let cat = Catalogue::open(&db).unwrap();
    let day = Duration::from_secs(24 * 3600);
    assert_eq!(cat.staleness(&root, day).unwrap(), Staleness::Missing);
    discover(&db, &root, &|_| {}).unwrap();
    assert!(matches!(
        cat.staleness(&root, day).unwrap(),
        Staleness::Fresh { .. }
    ));
    assert!(matches!(
        cat.staleness(&root, Duration::ZERO).unwrap(),
        Staleness::Fresh { .. } | Staleness::Expired { .. }
    ));
    assert_eq!(
        cat.staleness(Path::new("/elsewhere"), day).unwrap(),
        Staleness::RootChanged
    );
    cat.with(|c| {
        c.execute(
            "UPDATE meta SET value='1' WHERE key='last_scan_completed_at'",
            [],
        )
    })
    .unwrap();
    assert!(matches!(
        cat.staleness(&root, day).unwrap(),
        Staleness::Expired { .. }
    ));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reroutes_references_to_same_named_files_elsewhere() {
    let root = fixture();
    fs::create_dir_all(root.join("Sceneryobjects/PackA/model")).unwrap();
    fs::create_dir_all(root.join("Sceneryobjects/PackB/texture")).unwrap();
    fs::write(
        root.join("Sceneryobjects/PackA/a.sco"),
        "[mesh]\na.o3d\n[matl]\nlost.bmp\n0\n[matl]\ngone.bmp\n1\n",
    )
    .unwrap();
    fs::write(root.join("Sceneryobjects/PackA/model/a.o3d"), b"x").unwrap();
    fs::write(root.join("Sceneryobjects/PackB/texture/LOST.dds"), b"x").unwrap(); // other pack, other extension
    let db = root.join("cat.db");
    let summary = discover(&db, &root, &|_| {}).unwrap();
    assert_eq!(summary.rerouted, 1, "{summary:?}");
    assert_eq!(summary.unresolved, 1, "gone.bmp exists nowhere");
    let cat = Catalogue::open(&db).unwrap();
    let state: i64 = cat
        .with(|c| {
            c.query_row(
                "SELECT is_resolved FROM entity_references WHERE raw_reference='lost.bmp'",
                [],
                |r| r.get(0),
            )
        })
        .unwrap();
    assert_eq!(state, 2);
    let found = cat
        .find_by_name("textures\\lost.bmp", "Sceneryobjects/PackA", true)
        .unwrap();
    assert!(found.is_some_and(|p| p.ends_with("PackB/texture/LOST.dds")));
    assert!(cat.find_by_name("gone.bmp", "", true).unwrap().is_none());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn extracts_scenery_and_spline_references() {
    let root = fixture();
    fs::create_dir_all(root.join("Sceneryobjects/Pack/model")).unwrap();
    fs::create_dir_all(root.join("Sceneryobjects/Pack/texture")).unwrap();
    fs::create_dir_all(root.join("Splines/Road/texture")).unwrap();
    fs::create_dir_all(root.join("Texture")).unwrap();
    fs::write(
        root.join("Sceneryobjects/Pack/house.sco"),
        "[friendlyname]\nHouse\n[mesh]\nhouse.o3d\n[matl]\nwall.bmp\n0\n[matl]\nmissing.bmp\n1\n",
    )
    .unwrap();
    fs::write(root.join("Sceneryobjects/Pack/model/house.o3d"), b"x").unwrap();
    fs::write(root.join("Sceneryobjects/Pack/texture/wall.dds"), b"x").unwrap(); // other extension
    fs::write(
        root.join("Splines/Road/street.sli"),
        "[texture]\nasphalt.bmp\n",
    )
    .unwrap();
    fs::write(root.join("Texture/asphalt.bmp"), b"x").unwrap(); // global fallback
    let db = root.join("cat.db");
    discover(&db, &root, &|_| {}).unwrap();
    let cat = Catalogue::open(&db).unwrap();
    let rows: Vec<(String, String, bool)> = cat
        .with(|c| {
            let mut st = c.prepare(
                "SELECT role, raw_reference, is_resolved FROM entity_references ORDER BY role, raw_reference",
            )?;
            let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? == 1)))?;
            r.collect()
        })
        .unwrap();
    let has = |role: &str, raw: &str, ok: bool| {
        rows.iter().any(|r| r.0 == role && r.1 == raw && r.2 == ok)
    };
    assert!(has("model", "house.o3d", true), "{rows:?}");
    assert!(
        has("texture", "wall.bmp", true),
        "extension fallback: {rows:?}"
    );
    assert!(has("texture", "missing.bmp", false));
    assert!(
        has("texture", "asphalt.bmp", true),
        "global Texture/ fallback"
    );
    let _ = fs::remove_dir_all(&root);
}
