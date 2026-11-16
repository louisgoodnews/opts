//! Unit tests for `super` (`bus_config_spawn.rs`).

use super::*;
use crate::bus_config::Problem;
use crate::new_game_screen::types::VehicleItem;

fn catalogue() -> GameCatalogue {
    GameCatalogue {
        maps: Vec::new(),
        vehicles: vec![{
            let mut v = VehicleItem::named("c2", "Solo", "C2");
            v.bus_file = "Vehicles/C2/c2.bus".into();
            v
        }],
    }
}

fn config() -> BusConfiguration {
    BusConfiguration::new("Morning", "c2", "Red", vec![("Heater".into(), 1)], 0)
}

fn checked(problems: Vec<Problem>) -> Checked {
    Checked {
        repaint: "Red".into(),
        setvars: vec![("Heater".into(), 1)],
        problems,
    }
}

#[test]
fn a_usable_configuration_becomes_a_spawn_request() {
    let p = spawn_params(
        &config(),
        &checked(vec![]),
        &catalogue(),
        |f| Some(PathBuf::from(format!("/omsi/{f}"))),
        EntrySel::Id(3),
    )
    .expect("spawn");
    assert_eq!(p.catalogue_id, "c2");
    assert_eq!(p.name, catalogue().vehicles[0].full_name);
    assert_eq!(p.bus_file, "/omsi/Vehicles/C2/c2.bus");
    assert_eq!(p.repaint, "Red");
    assert_eq!(p.setvars, vec![("Heater".to_owned(), 1)]);
    assert_eq!(p.entry, EntrySel::Id(3));
}

#[test]
fn a_dropped_repaint_still_spawns_with_the_default() {
    let mut ch = checked(vec![Problem::RepaintMissing {
        repaint: "Red".into(),
    }]);
    ch.repaint.clear();
    let p = spawn_params(
        &config(),
        &ch,
        &catalogue(),
        |_| Some("/x".into()),
        EntrySel::Default,
    )
    .expect("spawn");
    assert!(p.repaint.is_empty());
}

#[test]
fn a_missing_vehicle_or_file_is_refused_with_a_reason() {
    let gone = checked(vec![Problem::VehicleMissing {
        vehicle: "c2".into(),
    }]);
    let e = spawn_params(
        &config(),
        &gone,
        &catalogue(),
        |_| Some("/x".into()),
        EntrySel::Default,
    )
    .unwrap_err();
    assert!(matches!(e, SpawnProblem::Unusable(_)));
    let e = spawn_params(
        &config(),
        &checked(vec![]),
        &catalogue(),
        |_| None,
        EntrySel::Default,
    )
    .unwrap_err();
    assert_eq!(e, SpawnProblem::BusFileMissing("Vehicles/C2/c2.bus".into()));
}

#[test]
fn a_bare_path_vehicle_has_no_configuration() {
    let mut registry = SessionVehicles::default();
    let id = registry.request_spawn(SpawnParams {
        catalogue_id: String::new(),
        name: "Bare".into(),
        bus_file: "/x.bus".into(),
        repaint: String::new(),
        setvars: Vec::new(),
        entry: EntrySel::Default,
        formation: None,
    });
    let mut rec = registry
        .records()
        .iter()
        .find(|r| r.id == id)
        .cloned()
        .expect("record");
    assert!(from_record("x", &rec, 0).is_none());
    rec.catalogue_id = "c2".into();
    rec.repaint = "Red".into();
    let c = from_record("x", &rec, 7).expect("config");
    assert_eq!(
        (c.vehicle.as_str(), c.repaint.as_str(), c.created),
        ("c2", "Red", 7)
    );
}
