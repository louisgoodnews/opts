//! Change-map planning and the control refusals outside a drive.


use super::*;


pub(super) fn catalogue_with_two_maps() -> opts_game::new_game_screen::types::GameCatalogue {
    use opts_game::new_game_screen::types::{GameCatalogue, MapItem, VehicleItem};
    let map = |id: &str, entries: &[&str]| MapItem {
        id: id.into(),
        name: id.into(),
        friendly_name: id.into(),
        description: String::new(),
        tile_count: 0,
        standard_depot: String::new(),
        years: String::new(),
        entry_points: entries.iter().map(|e| (*e).to_owned()).collect(),
    };
    GameCatalogue {
        maps: vec![
            map("Alpha", &["Depot"]),
            map("Beta", &["Marktplatz", "Hafen"]),
        ],
        vehicles: vec![VehicleItem::named("Bus1", "Maker", "Solo")],
    }
}

#[test]
pub(super) fn change_map_plans_carries_the_vehicle_and_restarts_the_session_on_the_other_map() {
    let mut app = session_app();
    app.insert_resource(catalogue_with_two_maps());
    app.insert_resource(opts_game::new_game_screen::types::NewGameSetup::default());
    let id = add_vehicle(&mut app, "Maker Solo", "Depot", 0.0);
    app.world_mut()
        .resource_mut::<SessionVehicles>()
        .set_active(Some(u32::try_from(id).expect("small id")));
    // The plan only: nothing changes.
    let plan = call(
        &mut app,
        "session.change_map",
        json!({"map": "beta", "entry": "hafen", "dry_run": true}),
    )
    .expect("dry run");
    assert_eq!(plan["map"], "Beta");
    assert_eq!(plan["entry_point"], "Hafen");
    assert_eq!(plan["vehicle"], "Maker Solo");
    assert_eq!(plan["started"], false);
    assert_eq!(
        app.world()
            .resource::<opts_game::new_game_screen::types::NewGameSetup>()
            .selected_map,
        opts_game::new_game_screen::types::NewGameSetup::default().selected_map,
        "a dry run writes nothing"
    );
    // Refusals: unknown map, unknown entry, bad vehicle word.
    for bad in [
        json!({"map": "Atlantis"}),
        json!({"map": "Beta", "entry": "Strand"}),
        json!({"map": "Beta", "vehicle": "maybe"}),
        json!({}),
    ] {
        let e = call(&mut app, "session.change_map", bad.clone()).expect_err("refused");
        assert_eq!(e.code, code::INVALID_PARAMS, "{bad}: {}", e.message);
    }
    // For real: the setup says what the loading pipeline then reads, and the session is entered again.
    let started = call(&mut app, "session.change_map", json!({"map": "Beta"})).expect("start");
    assert_eq!(started["started"], true);
    assert_eq!(started["entry_defaulted"], true);
    let setup = app
        .world()
        .resource::<opts_game::new_game_screen::types::NewGameSetup>()
        .clone();
    assert_eq!(
        (
            setup.selected_map.as_str(),
            setup.selected_entry_point.as_str()
        ),
        ("Beta", "Marktplatz")
    );
    assert_eq!(setup.selected_vehicle, "Maker Solo");
    assert!(!setup.no_vehicle);
}

#[test]
pub(super) fn change_map_is_not_ready_outside_a_drive_and_refuses_a_missing_vehicle_until_told_to_go_without()
{
    let mut app = base_app(Vec::new());
    app.insert_resource(catalogue_with_two_maps());
    app.insert_resource(opts_game::new_game_screen::types::NewGameSetup::default());
    let e = call(&mut app, "session.change_map", json!({"map": "Beta"})).expect_err("no drive");
    assert_eq!(e.code, code::NOT_READY);

    let mut app = session_app();
    app.insert_resource(catalogue_with_two_maps());
    app.insert_resource(opts_game::new_game_screen::types::NewGameSetup::default());
    let id = add_vehicle(&mut app, "Gone Bus", "Depot", 0.0);
    app.world_mut()
        .resource_mut::<SessionVehicles>()
        .set_active(Some(u32::try_from(id).expect("small id")));
    let e = call(&mut app, "session.change_map", json!({"map": "Beta"})).expect_err("vehicle");
    assert_eq!(e.code, code::NOT_FOUND, "{}", e.message);
    let ok = call(
        &mut app,
        "session.change_map",
        json!({"map": "Beta", "vehicle": "none"}),
    )
    .expect("without");
    assert_eq!(ok["vehicle"], Value::Null);
    assert!(
        app.world()
            .resource::<opts_game::new_game_screen::types::NewGameSetup>()
            .no_vehicle
    );
}
