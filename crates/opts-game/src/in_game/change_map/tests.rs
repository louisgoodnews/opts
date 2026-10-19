//! Tests of the carry-over rules of a map change.

use super::*;
use crate::session_vehicles::{Lifecycle, SessionVehicles, SpawnParams};

fn map(id: &str, friendly: &str, entries: &[&str]) -> MapItem {
    MapItem {
        id: id.into(),
        name: id.into(),
        friendly_name: friendly.into(),
        description: String::new(),
        tile_count: 0,
        standard_depot: String::new(),
        years: String::new(),
        entry_points: entries.iter().map(|e| (*e).to_owned()).collect(),
    }
}

fn catalogue() -> GameCatalogue {
    GameCatalogue {
        maps: vec![
            map("Berlin-Spandau", "Berlin Spandau", &["Depot", "Rathaus"]),
            map("Grundorf", "Grundorf", &["Dorfplatz"]),
            map("Grundorf Nord", "Grundorf Nord", &[]),
        ],
        vehicles: vec![
            VehicleItem::named("MAN_SD77", "MAN", "SD77"),
            VehicleItem::named("Citaro", "Mercedes", "Citaro"),
        ],
    }
}

fn record(catalogue_id: &str, name: &str, repaint: &str) -> SessionVehicleRecord {
    let mut registry = SessionVehicles::default();
    let id = registry.request_spawn(SpawnParams {
        catalogue_id: catalogue_id.into(),
        name: name.into(),
        bus_file: String::new(),
        repaint: repaint.into(),
        setvars: vec![("doors".into(), 1)],
        entry: crate::session_vehicles::EntrySel::Default,
        formation: None,
    });
    let mut r = registry.get(id).expect("record").clone();
    r.lifecycle = Lifecycle::Alive;
    r
}

fn inputs<'a>(cat: &'a GameCatalogue, active: Option<&'a SessionVehicleRecord>) -> PlanInputs<'a> {
    PlanInputs {
        catalogue: cat,
        active,
        others: vec!["Citaro".into()],
        clock: (
            CalendarDate::new(2026, 3, 14),
            8.0 * 3600.0 + 45.0 * 60.0,
            "Rain".into(),
        ),
    }
}

#[test]
fn the_vehicle_its_repaint_the_time_and_the_weather_are_carried_over() {
    let cat = catalogue();
    let r = record("MAN_SD77", "MAN SD77", "Blue");
    let p = plan(
        &inputs(&cat, Some(&r)),
        "grundorf",
        Some("dorfplatz"),
        VehicleChoice::Carry,
    )
    .expect("plan");
    assert_eq!(p.map, "Grundorf");
    assert_eq!(p.entry_point, "Dorfplatz");
    assert!(!p.entry_defaulted && !p.no_vehicle);
    assert_eq!(
        (p.vehicle_name.as_str(), p.repaint.as_str()),
        ("MAN SD77", "Blue")
    );
    assert_eq!(p.setvars, vec![("doors".to_owned(), 1)]);
    assert_eq!(
        (p.date, p.weather.as_str()),
        (CalendarDate::new(2026, 3, 14), "Rain")
    );
    assert_eq!(
        p.dropped,
        vec!["Citaro".to_owned()],
        "the other session vehicles are listed"
    );
    let mut setup = NewGameSetup::default();
    p.apply_to(&mut setup);
    assert_eq!((setup.hour, setup.minute), (8, 45));
    assert_eq!((setup.year, setup.month, setup.day), (2026, 3, 14));
    assert_eq!(setup.selected_map, "Grundorf");
    assert_eq!(setup.selected_vehicle, "MAN SD77");
    assert_eq!(setup.selected_livery, "Blue");
    assert_eq!(setup.custom_setvars.get("doors"), Some(&1));
    assert_eq!(setup.weather_preset, "Rain");
    assert!(!setup.no_vehicle);
}

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
    let mut setup = NewGameSetup::default();
    setup.selected_vehicle = "keep".into();
    p.apply_to(&mut setup);
    assert!(setup.no_vehicle);
    assert_eq!(
        setup.selected_vehicle, "keep",
        "the setup's own vehicle choice is left alone"
    );
}

#[test]
fn a_session_without_a_vehicle_stays_without_one() {
    let cat = catalogue();
    let p = plan(&inputs(&cat, None), "Grundorf", None, VehicleChoice::Carry).expect("plan");
    assert!(p.no_vehicle);
}

#[test]
fn the_plan_describes_itself_for_the_protocol() {
    let cat = catalogue();
    let r = record("MAN_SD77", "MAN SD77", "");
    let p = plan(
        &inputs(&cat, Some(&r)),
        "Grundorf",
        None,
        VehicleChoice::Carry,
    )
    .expect("plan");
    let d = p.describe();
    assert_eq!(d["map"], "Grundorf");
    assert_eq!(d["vehicle"], "MAN SD77");
    assert_eq!(d["time"], "08:45");
    assert_eq!(d["date"], "2026-03-14");
    assert_eq!(d["dropped"], serde_json::json!(["Citaro"]));
}
