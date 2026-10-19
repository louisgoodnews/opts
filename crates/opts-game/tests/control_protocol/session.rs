//! Session commands: vehicles, spawn, switch, despawn and respawn.

use super::*;

fn session_app() -> App {
    let mut app = base_app(Vec::new());
    app.add_plugins(SessionVehiclesPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app
}

/// A session vehicle of the registry with a simulation, standing at `x`.
fn add_vehicle(app: &mut App, name: &str, entry: &str, x: f32) -> u64 {
    let mut controller = VehicleController::standard_bus(12_000.0, 5.9);
    controller.sim.chassis.position = Vec2::new(x, 0.0);
    let entity = app
        .world_mut()
        .spawn((controller, InGameEntity, Transform::default()))
        .id();
    let id = app
        .world_mut()
        .resource_mut::<SessionVehicles>()
        .register_alive(
            SpawnParams {
                name: name.to_owned(),
                ..Default::default()
            },
            entity,
            Some((1, entry.to_owned())),
            Some(SpawnPoint {
                x: f64::from(x),
                y: 0.0,
                z: 0.0,
                heading_deg: 0.0,
            }),
            None,
        );
    app.world_mut()
        .entity_mut(entity)
        .insert(SessionVehicle(id));
    u64::from(id)
}

#[test]
fn session_vehicles_lists_the_registry_and_the_place_label() {
    let mut app = session_app();
    let empty = call(&mut app, "session.vehicles", json!({})).expect("empty");
    assert_eq!(empty["count"], 0);
    assert_eq!(empty["active"], Value::Null);
    assert_eq!(empty["place_label"], Value::Null);
    let a = add_vehicle(&mut app, "Alpha", "Depot", 0.0);
    let b = add_vehicle(&mut app, "Beta", "Stop", 50.0);
    call(&mut app, "session.switch", json!({ "id": a })).expect("switch");
    app.update();
    app.update();
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    assert_eq!(list["count"], 2);
    assert_eq!(list["active"], a);
    assert_eq!(list["place_label"], "Depot");
    let first = &list["vehicles"][0];
    assert_eq!(
        (
            first["name"].as_str(),
            first["spawn_entry_name"].as_str(),
            first["lifecycle"].as_str()
        ),
        (Some("Alpha"), Some("Depot"), Some("alive"))
    );
    assert_eq!(list["vehicles"][1]["id"], b);
    assert_eq!(list["vehicles"][1]["active"], false);
}

#[test]
fn session_spawn_queues_a_request_and_validates_its_parameters() {
    let dir = scratch("session-spawn");
    let mut app = base_app(vec![dir.clone()]);
    app.add_plugins(SessionVehiclesPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    let bus = dir.join("Test_Bus.bus");
    std::fs::write(&bus, "x").expect("bus file");
    let err = call(&mut app, "session.spawn", json!({})).expect_err("vehicle is required");
    assert_eq!(err.code, code::INVALID_PARAMS);
    let err = call(
        &mut app,
        "session.spawn",
        json!({ "vehicle": "No Such Bus" }),
    )
    .expect_err("unknown vehicle");
    assert_eq!(err.code, code::NOT_FOUND);
    let err = call(
        &mut app,
        "session.spawn",
        json!({ "vehicle": bus.display().to_string(), "setvars": [1] }),
    )
    .expect_err("setvars is an object");
    assert_eq!(err.code, code::INVALID_PARAMS);
    let ok = call(
        &mut app,
        "session.spawn",
        json!({
            "vehicle": bus.display().to_string(),
            "repaint": "Red",
            "setvars": { "Klima": 1 },
            "entry": "Depot",
        }),
    )
    .expect("queued");
    assert_eq!(ok["queued"], true);
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    let v = &list["vehicles"][0];
    assert_eq!(v["id"], ok["id"]);
    assert_eq!(v["lifecycle"], "spawning");
    assert_eq!(v["repaint"], "Red");
    assert_eq!(v["setvars"][0]["name"], "Klima");
    assert_eq!(v["entry"]["name"], "Depot");
    // A request that never existed in the world can be cancelled.
    let gone = call(&mut app, "session.despawn", json!({ "id": ok["id"] })).expect("cancel");
    assert_eq!(gone["ok"], true);
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    assert_eq!(list["vehicles"][0]["lifecycle"], "despawned");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn session_switch_moves_the_driver_to_another_vehicle() {
    let mut app = session_app();
    let a = add_vehicle(&mut app, "Alpha", "Depot", 0.0);
    let b = add_vehicle(&mut app, "Beta", "Stop", 50.0);
    let err = call(&mut app, "session.switch", json!({})).expect_err("needs id or step");
    assert_eq!(err.code, code::INVALID_PARAMS);
    let err = call(&mut app, "session.switch", json!({ "id": 99 })).expect_err("unknown");
    assert_eq!(err.code, code::NOT_FOUND);
    assert_eq!(
        call(&mut app, "session.switch", json!({ "id": a })).expect("a")["active"],
        a
    );
    assert_eq!(
        call(&mut app, "session.switch", json!({ "step": 1 })).expect("next")["active"],
        b
    );
    let mut q = app
        .world_mut()
        .query_filtered::<&SessionVehicle, With<PlayerVehicle>>();
    let active: Vec<u32> = q.iter(app.world()).map(|s| s.0).collect();
    assert_eq!(active, vec![b as u32], "only the new vehicle takes input");
    // The old one is parked.
    let mut q = app
        .world_mut()
        .query_filtered::<(&SessionVehicle, &VehicleController), Without<PlayerVehicle>>();
    let parked = q.iter(app.world()).next().expect("parked");
    assert!(parked.1.inputs().is_parking_brake_engaged());
}

#[test]
fn session_despawn_and_respawn_work_on_active_and_inactive_vehicles() {
    let mut app = session_app();
    let a = add_vehicle(&mut app, "Alpha", "Depot", 0.0);
    let b = add_vehicle(&mut app, "Beta", "Stop", 50.0);
    call(&mut app, "session.switch", json!({ "id": b })).expect("b");
    // Respawn of the active vehicle: back at its spawn point.
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
        q.single_mut(app.world_mut())
            .expect("player")
            .sim
            .chassis
            .position = Vec2::new(400.0, 3.0);
    }
    let r = call(&mut app, "session.respawn", json!({})).expect("respawn active");
    assert_eq!(r["id"], b);
    let mut q = app
        .world_mut()
        .query_filtered::<&VehicleController, With<PlayerVehicle>>();
    assert_eq!(
        q.single(app.world()).expect("p").sim.chassis.position,
        Vec2::new(50.0, 0.0)
    );
    // Despawn the inactive one.
    let out = call(&mut app, "session.despawn", json!({ "id": a })).expect("inactive");
    assert_eq!(out["active"], b);
    assert_eq!(out["removed"], 1);
    let err = call(&mut app, "session.respawn", json!({ "id": a })).expect_err("gone");
    assert_eq!(err.code, code::NOT_READY);
    // Despawn the active one: none is left.
    let out = call(&mut app, "session.despawn", json!({})).expect("active");
    assert_eq!(out["active"], Value::Null);
    let err = call(&mut app, "session.despawn", json!({})).expect_err("nothing to despawn");
    assert_eq!(err.code, code::NOT_READY);
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    assert_eq!(list["count"], 0);
    assert_eq!(list["place_label"], Value::Null);
}

fn catalogue_with_two_maps() -> opts_game::new_game_screen::types::GameCatalogue {
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
fn change_map_plans_carries_the_vehicle_and_restarts_the_session_on_the_other_map() {
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
fn change_map_is_not_ready_outside_a_drive_and_refuses_a_missing_vehicle_until_told_to_go_without()
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
