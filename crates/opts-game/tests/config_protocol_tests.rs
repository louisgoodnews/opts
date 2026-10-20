//! `config.*` (M-0077): saving the plan as a named configuration, listing it with its check,
//! loading it into the plan, and the refusals.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_game::bus_config::BusConfigurations;
use opts_game::control_configs::BusConfigFile;
use opts_game::new_game_screen::livery::LiveryState;
use opts_game::new_game_screen::planner::PlannerState;
use opts_game::new_game_screen::repaints::VehicleRepaints;
use opts_game::new_game_screen::types::{GameCatalogue, NewGameSetup, VehicleItem};
use opts_game::session_vehicles::{EntrySel, SessionVehicles, SpawnParams};
use serde_json::{json, Value};

fn reps() -> VehicleRepaints {
    let mut reps = VehicleRepaints {
        default_name: "Red".into(),
        names: vec!["Red".into(), "Blue".into()],
        folder: "Citybus 530 by Kajosoft".into(),
        ..VehicleRepaints::default()
    };
    reps.setvars.insert(
        "Red".into(),
        vec![
            ("vis_x".into(), "1".into()),
            ("display_ext".into(), "2".into()),
        ],
    );
    reps.setvars
        .insert("Blue".into(), vec![("display_ext".into(), "5".into())]);
    reps
}

struct Fixture {
    app: App,
    dir: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn fixture(name: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("opts-config-proto-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_configs::register(&mut registry);
    let mut liveries = LiveryState::default();
    liveries.vehicle = "citybus".into();
    liveries.repaints = Some(reps());
    app.insert_resource(registry)
        .insert_resource(GameCatalogue {
            maps: Vec::new(),
            vehicles: vec![
                VehicleItem::named("citybus", "Mercedes", "O530"),
                VehicleItem::named("solaris", "Solaris", "Urbino"),
            ],
        })
        .insert_resource(NewGameSetup {
            selected_vehicle: "Mercedes O530".into(),
            selected_livery: "Blue".into(),
            ..NewGameSetup::default()
        })
        .insert_resource(PlannerState {
            built: Some(Default::default()),
            ..default()
        })
        .insert_resource(liveries)
        .insert_resource(BusConfigFile(dir.join("bus_configurations.json")));
    Fixture { app, dir }
}

fn call(app: &mut App, method: &str, params: Value) -> Result<Value, RpcError> {
    match dispatch(app.world_mut(), method, &params) {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

fn setup(app: &App) -> NewGameSetup {
    app.world().resource::<NewGameSetup>().clone()
}

#[test]
fn a_saved_configuration_comes_back_into_the_plan_of_another_session() {
    let mut f = fixture("round-trip");
    f.app
        .world_mut()
        .resource_mut::<NewGameSetup>()
        .custom_setvars
        .insert("vis_x".into(), 0);
    let saved = call(
        &mut f.app,
        "config.save",
        json!({ "name": "Line 33", "notes": "late" }),
    )
    .expect("saves");
    assert_eq!(saved["id"], "line-33");
    assert_eq!(saved["replaced"], false);
    assert_eq!(saved["configuration"]["vehicle"], "citybus");
    assert_eq!(saved["configuration"]["vehicle_name"], "Mercedes O530");

    // Another session: the plan is something else.
    *f.app.world_mut().resource_mut::<NewGameSetup>() = NewGameSetup {
        selected_vehicle: "Solaris Urbino".into(),
        ..NewGameSetup::default()
    };
    let listed = call(&mut f.app, "config.list", json!({})).expect("lists");
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["configurations"][0]["usable"], true);
    assert_eq!(listed["configurations"][0]["problems"], json!([]));

    let loaded = call(&mut f.app, "config.load", json!({ "id": "line 33" })).expect("loads");
    assert_eq!(loaded["context"], "wizard");
    let s = setup(&f.app);
    assert_eq!(s.selected_vehicle, "Mercedes O530");
    assert_eq!(s.selected_livery, "Blue");
    assert_eq!(s.custom_setvars.get("vis_x"), Some(&0));
    // The wizard builds its body and the repaint data again.
    assert_eq!(f.app.world().resource::<PlannerState>().built, None);
    assert!(f.app.world().resource::<LiveryState>().repaints.is_none());
}

#[test]
fn a_taken_name_needs_replace_and_keeps_the_identity() {
    let mut f = fixture("replace");
    let first = call(&mut f.app, "config.save", json!({ "name": "Day" })).expect("saves");
    call(
        &mut f.app,
        "config.update",
        json!({ "id": "day", "favourite": true }),
    )
    .expect("marks");
    let e = call(&mut f.app, "config.save", json!({ "name": "DAY" })).expect_err("refused");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let again = call(
        &mut f.app,
        "config.save",
        json!({ "name": "DAY", "replace": true }),
    )
    .expect("replaces");
    assert_eq!(again["id"], first["id"]);
    assert_eq!(again["replaced"], true);
    let listed = call(&mut f.app, "config.list", json!({})).expect("lists");
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["configurations"][0]["favourite"], true);
}

#[test]
fn nothing_to_save_without_a_vehicle() {
    let mut f = fixture("no-vehicle");
    f.app.world_mut().resource_mut::<NewGameSetup>().no_vehicle = true;
    let e = call(&mut f.app, "config.save", json!({ "name": "x" })).expect_err("refused");
    assert_eq!(e.code, code::NOT_READY);
    let e = call(&mut f.app, "config.save", json!({ "name": "  " })).expect_err("refused");
    assert_eq!(e.code, code::INVALID_PARAMS);
}

#[test]
fn a_vehicle_that_is_gone_refuses_the_load_and_changes_nothing() {
    let mut f = fixture("gone");
    call(&mut f.app, "config.save", json!({ "name": "Old" })).expect("saves");
    // The bus is removed from the installation.
    f.app
        .world_mut()
        .resource_mut::<GameCatalogue>()
        .vehicles
        .retain(|v| v.id != "citybus");
    let before = setup(&f.app);
    let listed = call(&mut f.app, "config.list", json!({})).expect("lists");
    assert_eq!(listed["configurations"][0]["usable"], false);
    assert_eq!(
        listed["configurations"][0]["problems"][0]["code"],
        "vehicle_missing"
    );
    let e = call(&mut f.app, "config.load", json!({ "id": "old" })).expect_err("refused");
    assert_eq!(e.code, code::FAILED);
    assert!(e.message.contains("not installed"), "{}", e.message);
    assert_eq!(setup(&f.app), before);
}

#[test]
fn a_repaint_or_value_the_vehicle_no_longer_has_is_reported_and_left_out() {
    let mut f = fixture("partial");
    {
        let mut s = f.app.world_mut().resource_mut::<NewGameSetup>();
        s.custom_setvars.insert("display_ext".into(), 5);
        s.custom_setvars.insert("vis_x".into(), 0);
    }
    call(&mut f.app, "config.save", json!({ "name": "Mine" })).expect("saves");
    // The vehicle's repaints change: Blue is gone and display_ext only has 2.
    let mut changed = reps();
    changed.names = vec!["Red".into()];
    changed.setvars.remove("Blue");
    f.app.world_mut().resource_mut::<LiveryState>().repaints = Some(changed);
    let loaded = call(&mut f.app, "config.load", json!({ "id": "mine" })).expect("loads");
    let codes: Vec<&str> = loaded["problems"]
        .as_array()
        .expect("list")
        .iter()
        .filter_map(|p| p["code"].as_str())
        .collect();
    assert_eq!(codes, ["repaint_missing", "setvar_rejected"]);
    let s = setup(&f.app);
    assert_eq!(s.selected_livery, "");
    assert_eq!(s.custom_setvars.get("display_ext"), None);
    assert_eq!(s.custom_setvars.get("vis_x"), Some(&0));
}

#[test]
fn update_duplicate_and_delete() {
    let mut f = fixture("manage");
    call(&mut f.app, "config.save", json!({ "name": "A" })).expect("saves");
    call(
        &mut f.app,
        "config.save",
        json!({ "name": "B", "replace": true }),
    )
    .expect("saves");
    let e = call(
        &mut f.app,
        "config.update",
        json!({ "id": "a", "name": "b" }),
    )
    .expect_err("taken");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let e = call(&mut f.app, "config.update", json!({ "id": "a" })).expect_err("nothing to do");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let u = call(
        &mut f.app,
        "config.update",
        json!({ "id": "a", "name": "Alpha", "notes": "n" }),
    )
    .expect("updates");
    assert_eq!(u["configuration"]["name"], "Alpha");
    assert_eq!(u["configuration"]["id"], "a");
    let d = call(
        &mut f.app,
        "config.duplicate",
        json!({ "id": "alpha", "name": "Copy" }),
    )
    .expect("copies");
    assert_eq!(d["id"], "copy");
    assert_eq!(d["configuration"]["notes"], "n");
    call(&mut f.app, "config.delete", json!({ "id": "copy" })).expect("deletes");
    let e = call(&mut f.app, "config.delete", json!({ "id": "copy" })).expect_err("gone");
    assert_eq!(e.code, code::NOT_FOUND);
    let e = call(&mut f.app, "config.load", json!({ "id": "nothing" })).expect_err("unknown");
    assert_eq!(e.code, code::NOT_FOUND);
    assert_eq!(
        call(&mut f.app, "config.list", json!({})).expect("lists")["count"],
        2
    );
}

#[test]
fn a_damaged_or_newer_file_is_reported_not_overwritten() {
    let mut f = fixture("file");
    let path = f.dir.join("bus_configurations.json");
    std::fs::create_dir_all(&f.dir).expect("dir");
    std::fs::write(&path, r#"{"version":99,"configurations":[]}"#).expect("writes");
    let e = call(&mut f.app, "config.list", json!({})).expect_err("newer");
    assert_eq!(e.code, code::FAILED);
    let e = call(&mut f.app, "config.save", json!({ "name": "x" })).expect_err("newer");
    assert_eq!(e.code, code::FAILED);
    assert!(std::fs::read_to_string(&path)
        .expect("reads")
        .contains("99"));
    std::fs::write(&path, "{ broken").expect("writes");
    let e = call(&mut f.app, "config.list", json!({})).expect_err("damaged");
    assert!(e.message.contains("damaged"), "{}", e.message);
    // A save moves the damaged file aside and starts a new one.
    BusConfigurations::default().save_to(&path).expect("saves");
    assert!(f.dir.join("bus_configurations.json.broken").exists());
}

fn params(catalogue_id: &str, repaint: &str, setvars: Vec<(String, i32)>) -> SpawnParams {
    SpawnParams {
        catalogue_id: catalogue_id.into(),
        name: "x".into(),
        bus_file: "/x.bus".into(),
        repaint: repaint.into(),
        setvars,
        entry: EntrySel::Default,
        formation: None,
    }
}

#[test]
fn in_a_session_the_active_vehicle_is_what_gets_saved() {
    let mut f = fixture("session-save");
    let mut registry = SessionVehicles::default();
    let id = registry.register_alive(
        params("solaris", "Blue", vec![("display_ext".into(), 5)]),
        Entity::PLACEHOLDER,
        None,
        None,
        None,
    );
    registry.set_active(Some(id));
    f.app.insert_resource(registry);
    let saved = call(&mut f.app, "config.save", json!({ "name": "Second" })).expect("saves");
    assert_eq!(saved["configuration"]["vehicle"], "solaris");
    assert_eq!(saved["configuration"]["repaint"], "Blue");
    assert_eq!(saved["configuration"]["setvars"]["display_ext"], 5);
}

#[test]
fn spawning_a_configuration_needs_a_session_and_a_known_configuration() {
    let mut f = fixture("spawn-refusals");
    call(&mut f.app, "config.save", json!({ "name": "Line 33" })).expect("saves");
    let e = call(&mut f.app, "config.spawn", json!({ "id": "line-33" })).expect_err("no session");
    assert_eq!(e.code, code::NOT_READY);
    f.app.insert_resource(SessionVehicles::default());
    let e = call(&mut f.app, "config.spawn", json!({ "id": "nope" })).expect_err("unknown");
    assert_eq!(e.code, code::NOT_FOUND);
    let e = call(&mut f.app, "config.spawn", json!({ "id": "line-33" })).expect_err("no world");
    assert_eq!(e.code, code::NOT_READY);
    let e = call(&mut f.app, "config.spawn", json!({})).expect_err("no id");
    assert_eq!(e.code, code::INVALID_PARAMS);
}
