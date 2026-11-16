//! Discovery tests of the catalogue.

use super::*;

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
