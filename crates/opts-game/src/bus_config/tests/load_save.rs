//! Saving and restoring a bus configuration: the choices come back in another session, and a missing vehicle or repaint is handled.

use super::*;

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
