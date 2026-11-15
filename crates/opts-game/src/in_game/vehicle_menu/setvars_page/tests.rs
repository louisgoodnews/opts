//! Tests of the Setvars page: the options of the active vehicle and their toggles.

use super::*;
use crate::new_game_screen::repaints::VehicleRepaints;
use crate::session_vehicles::SpawnParams;
use std::collections::BTreeMap;

/// Repaint `Red`: `Heater` is a switch (0 or 1), `Seats` a choice of 1 to 3.
fn repaints() -> VehicleRepaints {
    let mut setvars = BTreeMap::new();
    setvars.insert(
        "Red".to_string(),
        vec![
            ("Heater".to_string(), "0".to_string()),
            ("Seats".to_string(), "1".to_string()),
        ],
    );
    setvars.insert(
        "Blue".to_string(),
        vec![
            ("Heater".to_string(), "1".to_string()),
            ("Seats".to_string(), "3".to_string()),
        ],
    );
    VehicleRepaints {
        default_name: "Red".into(),
        names: vec!["Red".into(), "Blue".into()],
        setvars,
        ..Default::default()
    }
}

fn app() -> App {
    let mut app = App::new();
    let mut registry = SessionVehicles::default();
    let id = registry.register_alive(
        SpawnParams {
            catalogue_id: "bus".into(),
            name: "Bus".into(),
            bus_file: "/bus.bus".into(),
            ..Default::default()
        },
        Entity::PLACEHOLDER,
        None,
        None,
        None,
    );
    registry.set_active(Some(id));
    app.insert_resource(registry).insert_resource(VehicleMenu {
        live_setvars: LiveSetvars {
            vehicle: Some(id),
            repaints: Some(repaints()),
            shown: None,
        },
        ..Default::default()
    });
    app
}

fn values(app: &App) -> Vec<(String, i32)> {
    let menu = app.world().resource::<VehicleMenu>();
    let registry = app.world().resource::<SessionVehicles>();
    items_of(menu, registry)
        .into_iter()
        .map(|i| (i.name, i.value))
        .collect()
}

#[test]
fn the_page_lists_the_options_with_the_repaints_values() {
    let app = app();
    let v = values(&app);
    assert!(v.contains(&("Heater".to_string(), 0)), "{v:?}");
    assert!(v.contains(&("Seats".to_string(), 1)), "{v:?}");
}

#[test]
fn toggling_and_stepping_change_the_record_and_reset_undoes_them() {
    let mut app = app();
    change(app.world_mut(), "Heater", None);
    change(app.world_mut(), "Seats", Some(1));
    let v = values(&app);
    assert!(v.contains(&("Heater".to_string(), 1)), "{v:?}");
    assert!(v.contains(&("Seats".to_string(), 3)), "{v:?}");
    // Both are kept on the session record, which the save and the configurations read.
    let record = app.world().resource::<SessionVehicles>().active().cloned();
    assert_eq!(record.map(|r| r.setvars.len()), Some(2));

    reset(app.world_mut());
    let v = values(&app);
    assert!(v.contains(&("Heater".to_string(), 0)), "{v:?}");
    assert!(v.contains(&("Seats".to_string(), 1)), "{v:?}");
    let record = app.world().resource::<SessionVehicles>().active().cloned();
    assert_eq!(record.map(|r| r.setvars.len()), Some(0));
}
