use std::collections::BTreeMap;

use super::*;

fn catalogue() -> GameCatalogue {
    GameCatalogue {
        maps: Vec::new(),
        vehicles: vec![
            VehicleItem::named("citaro", "Mercedes", "Citaro"),
            VehicleItem::named("lions", "MAN", "Lion's City"),
        ],
    }
}

/// Repaints Default and Red; the option `Klima` is a switch (0 or 1), `Sitze` a choice of 1 to 3.
fn repaints(_: &VehicleItem) -> VehicleRepaints {
    let mut setvars = BTreeMap::new();
    setvars.insert(
        "Default".to_string(),
        vec![
            ("Klima".to_string(), "0".to_string()),
            ("Sitze".to_string(), "1".to_string()),
        ],
    );
    setvars.insert(
        "Red".to_string(),
        vec![
            ("Klima".to_string(), "1".to_string()),
            ("Sitze".to_string(), "3".to_string()),
        ],
    );
    VehicleRepaints {
        default_name: "Default".into(),
        names: vec!["Default".into(), "Red".into()],
        setvars,
        ..Default::default()
    }
}

fn setup() -> NewGameSetup {
    let mut s = NewGameSetup {
        selected_vehicle: "Mercedes Citaro".into(),
        selected_livery: "Red".into(),
        ..NewGameSetup::default()
    };
    s.custom_setvars.insert("Klima".into(), 1);
    s
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("opts-bus-config-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

#[test]
fn a_saved_configuration_restores_the_same_choices_in_another_session() {
    let cat = catalogue();
    let config = BusConfiguration::from_setup("Morning shift", &cat, &setup(), 100).expect("saved");
    assert_eq!(config.vehicle, "citaro");
    let mut store = BusConfigurations::default();
    let (id, replaced) = store.save(config);
    assert_eq!((id.as_str(), replaced), ("morning-shift", false));

    let d = dir("round-trip");
    let path = d.join(FILE_NAME);
    store.save_to(&path).expect("writes");
    let loaded = BusConfigurations::load_from(&path).expect("reads");
    assert_eq!(loaded.version, FORMAT_VERSION);

    let found = loaded.find("morning-shift").expect("listed");
    let checked = found.check(&cat, repaints);
    assert!(checked.problems.is_empty(), "{:?}", checked.problems);
    let mut other = NewGameSetup::default();
    assert!(found.apply_to(&cat, &checked, &mut other).is_some());
    assert_eq!(other.selected_vehicle, "Mercedes Citaro");
    assert_eq!(other.selected_livery, "Red");
    assert_eq!(other.custom_setvars.get("Klima"), Some(&1));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn no_vehicle_or_an_unknown_one_cannot_be_saved() {
    let cat = catalogue();
    let none = NewGameSetup {
        no_vehicle: true,
        ..setup()
    };
    assert!(BusConfiguration::from_setup("x", &cat, &none, 1).is_none());
    let unknown = NewGameSetup {
        selected_vehicle: "Nothing".into(),
        ..setup()
    };
    assert!(BusConfiguration::from_setup("x", &cat, &unknown, 1).is_none());
}

#[test]
fn a_missing_vehicle_makes_the_configuration_unusable_and_writes_nothing() {
    let cat = catalogue();
    let config = BusConfiguration::new("Old bus", "gone", "", Vec::new(), 1);
    let checked = config.check(&cat, repaints);
    assert!(!checked.usable());
    assert_eq!(
        checked.problems,
        vec![Problem::VehicleMissing {
            vehicle: "gone".into()
        }]
    );
    let mut setup = NewGameSetup::default();
    let before = setup.clone();
    assert!(config.apply_to(&cat, &checked, &mut setup).is_none());
    assert_eq!(setup, before);
}

#[test]
fn a_missing_repaint_falls_back_and_a_refused_value_is_reported_not_applied() {
    let cat = catalogue();
    let config = BusConfiguration::new(
        "Changed",
        "citaro",
        "Green",
        vec![
            ("Klima".into(), 1),
            ("Sitze".into(), 9),
            ("Zugzielanzeige".into(), 2),
        ],
        1,
    );
    let checked = config.check(&cat, repaints);
    assert!(checked.usable());
    assert_eq!(checked.repaint, "");
    assert_eq!(checked.setvars, vec![("Klima".to_string(), 1)]);
    let codes: Vec<_> = checked.problems.iter().map(Problem::code).collect();
    assert_eq!(
        codes,
        ["repaint_missing", "setvar_rejected", "setvar_unknown"]
    );
    assert!(checked.problems[0].to_string().contains("Green"));
}

#[test]
fn a_file_before_the_version_field_and_with_fields_missing_still_loads() {
    let d = dir("old");
    let path = d.join(FILE_NAME);
    std::fs::write(
        &path,
        r#"{"configurations":[{"id":"a","name":"A","vehicle":"citaro"}]}"#,
    )
    .expect("writes");
    let store = BusConfigurations::load_from(&path).expect("reads");
    assert_eq!(store.version, FORMAT_VERSION);
    let a = store.find("a").expect("listed");
    assert_eq!(
        (a.repaint.as_str(), a.setvars.len(), a.favourite, a.created),
        ("", 0, false, 0)
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_newer_file_is_reported_and_left_alone() {
    let d = dir("newer");
    let path = d.join(FILE_NAME);
    let text = r#"{"version":99,"configurations":[]}"#;
    std::fs::write(&path, text).expect("writes");
    assert_eq!(
        BusConfigurations::load_from(&path),
        Err(StoreProblem::Newer(99))
    );
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), text);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_damaged_file_is_reported_and_kept_aside_when_the_store_is_saved() {
    let d = dir("damaged");
    let path = d.join(FILE_NAME);
    std::fs::write(&path, "{ not json").expect("writes");
    assert!(matches!(
        BusConfigurations::load_from(&path),
        Err(StoreProblem::Corrupt(_))
    ));
    BusConfigurations::default().save_to(&path).expect("saves");
    assert_eq!(
        std::fs::read_to_string(d.join(format!("{FILE_NAME}.broken"))).expect("kept"),
        "{ not json"
    );
    assert!(BusConfigurations::load_from(&path).is_ok());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_missing_file_is_an_empty_store() {
    let d = dir("missing");
    let store = BusConfigurations::load_from(&d.join(FILE_NAME)).expect("empty");
    assert!(store.configurations.is_empty());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn saving_under_a_taken_name_replaces_and_keeps_identity() {
    let mut store = BusConfigurations::default();
    let mut first = BusConfiguration::new("Day", "citaro", "", Vec::new(), 10);
    first.notes = "line 33".into();
    let (id, _) = store.save(first);
    store.set_favourite(&id, true);
    let (again, replaced) =
        store.save(BusConfiguration::new("DAY", "lions", "Red", Vec::new(), 50));
    assert!(replaced);
    assert_eq!(again, id);
    assert_eq!(store.configurations.len(), 1);
    let c = &store.configurations[0];
    assert_eq!(
        (c.vehicle.as_str(), c.created, c.last_used),
        ("lions", 10, 50)
    );
    assert_eq!(c.notes, "line 33");
    assert!(c.favourite);
}

#[test]
fn ids_are_unique_slugs() {
    let mut store = BusConfigurations::default();
    let mut add = |name: &str| {
        store
            .save(BusConfiguration::new(name, "citaro", "", Vec::new(), 1))
            .0
    };
    assert_eq!(add("Früh / Spät"), "früh-spät");
    // A different name with the same slug is numbered; the same name ignoring case replaces.
    assert_eq!(add("Früh - Spät"), "früh-spät-2");
    assert_eq!(add("FRÜH / SPÄT"), "früh-spät");
    assert_eq!(add("///"), "configuration");
    assert_eq!(store.configurations.len(), 3);
}

#[test]
fn rename_duplicate_and_delete() {
    let mut store = BusConfigurations::default();
    store.save(BusConfiguration::new("A", "citaro", "", Vec::new(), 1));
    store.save(BusConfiguration::new("B", "lions", "", Vec::new(), 1));
    assert!(!store.rename("a", "b"), "name taken");
    assert!(!store.rename("a", "  "), "empty name");
    assert!(store.rename("a", "Alpha"));
    assert_eq!(store.find("alpha").map(|c| c.id.as_str()), Some("a"));
    assert_eq!(
        store.duplicate("alpha", "Alpha copy", 9).as_deref(),
        Some("alpha-copy")
    );
    assert!(
        store.duplicate("alpha", "alpha COPY", 9).is_none(),
        "name taken"
    );
    assert!(store.duplicate("missing", "X", 9).is_none());
    assert!(store.delete("alpha-copy"));
    assert!(!store.delete("alpha-copy"));
    assert_eq!(store.configurations.len(), 2);
}
