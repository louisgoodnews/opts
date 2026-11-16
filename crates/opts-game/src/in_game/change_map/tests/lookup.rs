//! Finding the map, its entry point and the vehicle a change needs.

use super::*;

#[test]
fn the_first_entry_point_is_used_when_none_is_asked_for_and_a_map_without_entries_has_none() {
    let cat = catalogue();
    let r = record("Citaro", "Mercedes Citaro", "");
    let p = plan(
        &inputs(&cat, Some(&r)),
        "Berlin",
        None,
        VehicleChoice::Carry,
    )
    .expect("plan");
    assert_eq!((p.entry_point.as_str(), p.entry_defaulted), ("Depot", true));
    let q = plan(
        &inputs(&cat, Some(&r)),
        "grundorf nord",
        None,
        VehicleChoice::Carry,
    )
    .expect("plan");
    assert_eq!(
        q.entry_point, "",
        "the loading pipeline then finds its own start"
    );
}

#[test]
fn a_map_is_found_by_id_name_or_a_unique_part_and_a_shared_part_is_refused() {
    let cat = catalogue();
    let r = record("Citaro", "Mercedes Citaro", "");
    let i = inputs(&cat, Some(&r));
    assert_eq!(
        plan(&i, "Berlin-Spandau", None, VehicleChoice::Carry)
            .expect("id")
            .map,
        "Berlin Spandau"
    );
    assert_eq!(
        plan(&i, "rlin spa", None, VehicleChoice::Carry)
            .expect("part")
            .map_id,
        "Berlin-Spandau"
    );
    assert_eq!(
        plan(&i, "Grundorf Nord", None, VehicleChoice::Carry)
            .expect("exact")
            .map,
        "Grundorf Nord"
    );
    assert!(matches!(
        plan(&i, "grund", None, VehicleChoice::Carry),
        Err(ChangeMapError::AmbiguousMap(_, v)) if v.len() == 2
    ));
    assert!(matches!(
        plan(&i, "Atlantis", None, VehicleChoice::Carry),
        Err(ChangeMapError::UnknownMap(_))
    ));
}

#[test]
fn an_entry_point_the_map_does_not_have_is_refused_with_the_list() {
    let cat = catalogue();
    let r = record("Citaro", "Mercedes Citaro", "");
    let e = plan(
        &inputs(&cat, Some(&r)),
        "Grundorf",
        Some("Hafen"),
        VehicleChoice::Carry,
    )
    .expect_err("no such entry");
    assert_eq!(
        e,
        ChangeMapError::UnknownEntry {
            map: "Grundorf".into(),
            entry: "Hafen".into(),
            available: vec!["Dorfplatz".into()]
        }
    );
    assert!(e.to_string().contains("Dorfplatz"));
}

#[test]
fn a_vehicle_that_is_not_in_the_catalogue_is_asked_about_and_none_goes_on_without() {
    let cat = catalogue();
    let r = record("Gone_Bus", "Gone Bus", "");
    let i = inputs(&cat, Some(&r));
    assert_eq!(
        plan(&i, "Grundorf", None, VehicleChoice::Carry),
        Err(ChangeMapError::VehicleUnavailable("Gone Bus".into()))
    );
    let p = plan(&i, "Grundorf", None, VehicleChoice::None).expect("without a vehicle");
    assert!(p.no_vehicle && p.vehicle_key.is_empty() && p.vehicle_name.is_empty());
    let mut setup = NewGameSetup {
        selected_vehicle: "keep".into(),
        ..NewGameSetup::default()
    };
    p.apply_to(&mut setup);
    assert!(setup.no_vehicle);
    assert_eq!(
        setup.selected_vehicle, "keep",
        "the setup's own vehicle choice is left alone"
    );
}
