//! The control protocol's game-side commands, headless: the command table covers exactly what is
//! registered, driving controls pass the input gates, the odometer and placement commands work,
//! `ui.wait_for` waits and times out, and `snapshot` collects the state.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::{TimeUpdateStrategy, Virtual};
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::control::{ControlConfig, ControlPlugin};
use opts_bevy::reporting::state::ModalActive;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::{
    CameraMode, CameraState, OptsVehiclePlugin, PlayerVehicle, ScriptKeyBindings, VehicleController,
};
use opts_cli::commands::COMMANDS;
use opts_cli::protocol::{code, RpcError};
use opts_game::app_state::AppState;
use opts_game::control::GameControlPlugin;
use opts_game::in_game::omsi_world::WorldStats;
use opts_sim::vehicle::inputs::GearPosition;
use serde_json::{json, Value};

/// Descriptors of commands that the table promises for later and no build implements yet.
const NOT_IMPLEMENTED: &[&str] = &[
    "auth",
    "entity.list",
    "entity.get",
    "resource.list",
    "resource.get",
    "asset.list",
    "asset.get",
    "sim.set_time",
    "entity.spawn",
    "entity.despawn",
    "entity.set_component",
    "debug.wireframe",
    "debug.normals",
    "debug.stats",
];

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(std::env::var_os("HOME").expect("HOME"))
        .join(".local/share/opts/benchmark/control-protocol/test-scratch")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn base_app(allow: Vec<PathBuf>) -> App {
    opts_game::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, StatesPlugin));
    app.init_state::<AppState>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    app.add_plugins(ControlPlugin::new(ControlConfig {
        allow_dirs: allow,
        ..ControlConfig::default()
    }));
    app.add_plugins(GameControlPlugin);
    opts_game::settings_registry::protocol::register(&mut app);
    app.init_resource::<ScriptKeyBindings>()
        .init_resource::<WorldStats>()
        .init_resource::<CameraState>();
    app.update();
    app
}

fn drive_app() -> App {
    let mut app = base_app(Vec::new());
    app.add_plugins(OptsVehiclePlugin);
    app.world_mut().spawn((
        VehicleController::standard_bus(12_000.0, 5.9),
        PlayerVehicle,
    ));
    // Free roam is the default camera and takes the vehicle's input away; drive from outside.
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::External;
    app.update();
    app
}

fn call(app: &mut App, method: &str, params: Value) -> Result<Value, RpcError> {
    match dispatch(app.world_mut(), method, &params) {
        Outcome::Done(r) => r,
        Outcome::Wait(mut w) => {
            for _ in 0..4000 {
                if let Some(r) = w(app.world_mut()) {
                    return r;
                }
                app.update();
            }
            panic!("`{method}` never finished");
        }
    }
}

fn inputs(app: &mut App) -> (f32, f32, f32, GearPosition, bool) {
    let mut q = app
        .world_mut()
        .query_filtered::<&VehicleController, With<PlayerVehicle>>();
    let c = q.iter(app.world()).next().expect("vehicle");
    let i = c.inputs();
    (
        i.throttle(),
        i.service_brake(),
        i.steering(),
        i.gear(),
        i.is_parking_brake_engaged(),
    )
}

#[test]
fn the_command_table_and_the_registry_agree() {
    let app = base_app(Vec::new());
    let registry = app.world().resource::<ControlRegistry>();
    let registered = registry.methods();
    for m in &registered {
        assert!(
            COMMANDS.iter().any(|c| c.method == *m),
            "`{m}` is registered but has no descriptor in crates/opts-cli/src/commands.rs"
        );
    }
    for c in COMMANDS {
        let implemented = registered.contains(&c.method);
        let promised_only = NOT_IMPLEMENTED.contains(&c.method);
        assert!(
            implemented != promised_only,
            "`{}`: implemented={implemented}, listed as not implemented={promised_only}",
            c.method
        );
    }
}

#[test]
fn help_and_commands_describe_the_schema() {
    let mut app = base_app(Vec::new());
    let h = call(&mut app, "help", json!({"command": "drive.set"})).expect("help");
    assert_eq!(h["since"], "1.1");
    assert_eq!(h["implemented"], true);
    assert_eq!(h["args"][0]["name"], "control");
    assert_eq!(h["args"][1]["name"], "value");
    let all = call(&mut app, "commands", json!({"group": "drive"})).expect("commands");
    assert_eq!(all["protocol_version"], "1.2.0");
    let names: Vec<&str> = all["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["method"].as_str())
        .collect();
    for want in ["drive.set", "drive.trigger", "drive.state", "vehicle.reset"] {
        assert!(names.contains(&want), "{want} in {names:?}");
    }
    assert!(all["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["name"] == "GATED"));
    let none = call(&mut app, "commands", json!({"group": "nonsense"})).expect_err("group");
    assert_eq!(none.code, code::NOT_FOUND);
    assert!(none.data.expect("data")["groups"].is_array());
}

#[test]
fn drive_controls_reach_the_vehicle_through_the_input_system() {
    let mut app = drive_app();
    assert!(inputs(&mut app).4, "starts parked");
    call(
        &mut app,
        "drive.set",
        json!({"control": "gear", "value": "drive"}),
    )
    .expect("gear");
    call(
        &mut app,
        "drive.set",
        json!({"control": "parking_brake", "value": false}),
    )
    .expect("pb");
    call(
        &mut app,
        "drive.set",
        json!({"control": "throttle", "value": 0.6, "hold_ms": 100}),
    )
    .expect("throttle");
    call(
        &mut app,
        "drive.set",
        json!({"control": "steer", "value": -0.4}),
    )
    .expect("steer");
    app.update();
    let (throttle, _, steer, gear, parked) = inputs(&mut app);
    assert!((throttle - 0.6).abs() < 1e-5, "throttle {throttle}");
    assert!((steer + 0.4).abs() < 1e-5, "steer {steer}");
    assert_eq!(gear, GearPosition::Drive);
    assert!(!parked);
    // 100 ms of simulation time later the hold has run out and the pedal decays on its own.
    for _ in 0..30 {
        app.update();
    }
    assert!(
        inputs(&mut app).0 < 0.2,
        "throttle released after its duration"
    );
    assert!(
        (inputs(&mut app).2 + 0.4).abs() < 1e-5,
        "an open-ended hold stays"
    );
    call(&mut app, "drive.release", json!({})).expect("release");
    for _ in 0..200 {
        app.update();
    }
    assert!(
        inputs(&mut app).2.abs() < 0.05,
        "steering returns after release"
    );
    let controls = call(&mut app, "drive.controls", json!({})).expect("controls");
    assert!(controls["controls"].as_array().unwrap().len() >= 5);
}

#[test]
fn drive_controls_are_refused_by_the_same_gates_as_keys() {
    let mut app = drive_app();
    // A dialog or the vehicle menu is open.
    app.insert_resource(ModalActive(true));
    let e = call(
        &mut app,
        "drive.set",
        json!({"control": "throttle", "value": 1.0}),
    )
    .expect_err("gated");
    assert_eq!(e.code, code::GATED);
    assert_eq!(e.structured().data.expect("data")["name"], "GATED");
    app.insert_resource(ModalActive(false));
    // Free roam: the vehicle takes no input.
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::FreeRoam;
    let e = call(
        &mut app,
        "drive.set",
        json!({"control": "gear", "value": "drive"}),
    )
    .expect_err("free roam");
    assert_eq!(e.code, code::GATED);
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::External;
    // A held value is dropped when a gate closes, so nothing fires when it reopens.
    call(
        &mut app,
        "drive.set",
        json!({"control": "brake", "value": 1.0}),
    )
    .expect("brake");
    app.insert_resource(ModalActive(true));
    app.update();
    assert!(!app.world().resource::<RemoteControls>().is_active());
    app.insert_resource(ModalActive(false));
    app.update();
    assert!(
        inputs(&mut app).1 < 0.01,
        "the held brake did not come back"
    );
    // Bad values are invalid params, not gates.
    let bad = call(
        &mut app,
        "drive.set",
        json!({"control": "throttle", "value": 2}),
    )
    .expect_err("range");
    assert_eq!(bad.code, code::INVALID_PARAMS);
    let bad = call(
        &mut app,
        "drive.set",
        json!({"control": "gear", "value": "sideways"}),
    )
    .expect_err("gear");
    assert_eq!(bad.code, code::INVALID_PARAMS);
}

#[test]
fn without_a_vehicle_driving_commands_say_not_ready() {
    let mut app = base_app(Vec::new());
    for (m, p) in [
        ("drive.set", json!({"control": "throttle", "value": 0.5})),
        ("drive.trigger", json!({"name": "horn"})),
        ("drive.state", json!({})),
        ("drive.controls", json!({})),
        ("vehicle.reset", json!({})),
    ] {
        let e = call(&mut app, m, p).expect_err(m);
        assert_eq!(e.code, code::NOT_READY, "{m}");
    }
}

#[test]
fn odometer_teleport_reset_and_state() {
    let mut app = drive_app();
    let set_y = |app: &mut App, y: f32| {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
        q.iter_mut(app.world_mut())
            .next()
            .unwrap()
            .sim
            .chassis
            .position = Vec2::new(0.0, y);
    };
    for i in 1..=10 {
        set_y(&mut app, i as f32);
        app.update();
    }
    let state = call(&mut app, "drive.state", json!({})).expect("state");
    assert!(
        (state["distance_m"].as_f64().unwrap() - 10.0).abs() < 0.5,
        "{state}"
    );
    assert!(
        state["route"].is_null() && state["stops"].is_null(),
        "timetable is not guessed"
    );
    // A jump is a placement, not driving.
    set_y(&mut app, 500.0);
    app.update();
    let jumped = call(&mut app, "drive.state", json!({})).expect("state");
    assert!((jumped["distance_m"].as_f64().unwrap() - 10.0).abs() < 0.5);
    // Teleport: absolute coordinates, standing still, odometer restarts.
    let t = call(
        &mut app,
        "vehicle.teleport",
        json!({"x": 200.0, "y": 100.0, "heading_deg": 90.0}),
    )
    .expect("tp");
    assert_eq!(t["ok"], true);
    app.update();
    let s = call(&mut app, "drive.state", json!({})).expect("state");
    assert!((s["x"].as_f64().unwrap() - 200.0).abs() < 0.01);
    assert!((s["y"].as_f64().unwrap() - 100.0).abs() < 0.01);
    assert!((s["heading_deg"].as_f64().unwrap() - 90.0).abs() < 0.01);
    assert!(s["distance_m"].as_f64().unwrap() < 0.01);
    assert!(s["speed_kmh"].as_f64().unwrap() < 0.01);
    // Reset: back to where the session placed the vehicle (the origin here).
    call(&mut app, "vehicle.reset", json!({})).expect("reset");
    app.update();
    let r = call(&mut app, "drive.state", json!({})).expect("state");
    assert!(
        r["x"].as_f64().unwrap().abs() < 0.01 && r["y"].as_f64().unwrap().abs() < 0.01,
        "{r}"
    );
}

#[test]
fn wait_for_holds_on_conditions_and_times_out_with_the_last_value() {
    let mut app = drive_app();
    // Paused clock: the condition holds at once.
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    let ok = call(&mut app, "ui.wait_for", json!({"until": "paused"})).expect("paused");
    assert_eq!(ok["ok"], true);
    // A distance that is not driven times out with what was observed.
    let e = call(
        &mut app,
        "ui.wait_for",
        json!({"until": "distance_above", "number": 100.0, "timeout_ms": 60}),
    );
    // The waiter polls once per frame; give it real time to pass the deadline.
    let e = match e {
        Err(e) => e,
        Ok(v) => panic!("should time out: {v}"),
    };
    assert_eq!(e.code, code::TIMEOUT);
    assert_eq!(e.structured().data.expect("data")["name"], "TIMEOUT");
    // Invalid conditions are rejected up front, with the valid ones listed.
    let bad = call(&mut app, "ui.wait_for", json!({"until": "soon"})).expect_err("bad");
    assert_eq!(bad.code, code::INVALID_PARAMS);
    assert!(bad.data.expect("data")["valid"].as_array().unwrap().len() > 10);
    let missing =
        call(&mut app, "ui.wait_for", json!({"until": "speed_above"})).expect_err("number");
    assert_eq!(missing.code, code::INVALID_PARAMS);
    // app_state is read case-insensitively.
    let s = call(
        &mut app,
        "ui.wait_for",
        json!({"until": "app_state", "value": "loading"}),
    )
    .expect("state");
    assert_eq!(s["ok"], true);
}

#[test]
fn snapshot_collects_state_screen_drive_and_warnings_into_a_file() {
    let dir = scratch("snapshot");
    let mut app = base_app(vec![dir.clone()]);
    app.add_plugins(OptsVehiclePlugin);
    app.world_mut().spawn((
        VehicleController::standard_bus(12_000.0, 5.9),
        PlayerVehicle,
    ));
    app.update();
    let path = dir.join("snap.json");
    let v = call(
        &mut app,
        "snapshot",
        json!({"path": path.display().to_string()}),
    )
    .expect("snapshot");
    for key in ["state", "screen", "drive", "camera", "sim", "warnings"] {
        assert!(v.get(key).is_some(), "snapshot has `{key}`: {v}");
    }
    assert_eq!(v["screen"]["app_state"], "Loading");
    let on_disk: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("file")).expect("json");
    assert_eq!(on_disk["drive"]["gear"], v["drive"]["gear"]);
    // Writing outside the allowed directories is refused.
    let e = call(&mut app, "snapshot", json!({"path": "/proc/nope.json"})).expect_err("forbidden");
    assert!(e.code == code::FORBIDDEN_PATH || e.code == code::NOT_FOUND);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn screen_reports_state_without_dialogs() {
    let mut app = base_app(Vec::new());
    let s = call(&mut app, "ui.screen", json!({})).expect("screen");
    assert_eq!(s["app_state"], "Loading");
    assert_eq!(s["modal_open"], false);
    assert!(s["dialogs"].as_array().unwrap().is_empty());
    app.insert_resource(ModalActive(true));
    let s = call(&mut app, "ui.screen", json!({})).expect("screen");
    assert_eq!(s["modal_open"], true, "the report modal counts");
}

fn variable(app: &mut App, name: &str) -> Option<f32> {
    let mut q = app
        .world_mut()
        .query_filtered::<&opts_bevy::VehicleScripts, With<PlayerVehicle>>();
    let scripts = q.iter(app.world()).next()?;
    let engine = scripts.engine.lock().ok()?;
    engine
        .variables()
        .into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v)
}

const SCRIPT: &str = "\
{trigger:horn}
    1 (S.L.hornv)
{end}

{trigger:horn_off}
    0 (S.L.hornv)
{end}

{trigger:bus_doorfront0}
    1 (S.L.doorv)
{end}
";

#[test]
fn triggers_raise_script_events_with_aliases_holds_and_gates() {
    let mut app = drive_app();
    let keyboard = opts_core::formats::engine_config::KeyboardConfig::parse(
        b"[vehicles]\n[entry]\nbus_doorfront0\n181\n0\n[entry]\nhorn\n35\n1\n",
    );
    app.insert_resource(ScriptKeyBindings::from_config(&keyboard));
    let (mut engine, _) = opts_sim::script::ScriptEngine::from_sources(&[("t.osc", SCRIPT)]);
    engine.run_init();
    let vehicle = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PlayerVehicle>>();
        q.iter(app.world()).next().expect("vehicle")
    };
    app.world_mut()
        .entity_mut(vehicle)
        .insert(opts_bevy::VehicleScripts::new(engine));
    // A held binding (the horn): `horn` now, `horn_off` after the hold, in simulation time.
    let horn = call(
        &mut app,
        "drive.trigger",
        json!({"name": "horn", "hold_ms": 100}),
    )
    .expect("horn");
    assert_eq!(horn["handled"], true);
    assert_eq!(horn["held_ms"], 100);
    assert_eq!(variable(&mut app, "hornv"), Some(1.0));
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        variable(&mut app, "hornv"),
        Some(0.0),
        "horn_off followed the hold"
    );
    // A short name resolves to the vehicle's own trigger; `drive.set` takes trigger names too.
    let door = call(
        &mut app,
        "drive.set",
        json!({"control": "door_front", "value": true}),
    )
    .expect("door");
    assert_eq!(door["name"], "bus_doorfront0");
    assert_eq!(variable(&mut app, "doorv"), Some(1.0));
    // A name no script handles is raised and reported as not handled.
    let none = call(
        &mut app,
        "drive.trigger",
        json!({"name": "no_such_trigger"}),
    )
    .expect("raise");
    assert_eq!(none["handled"], false);
    // The modal gate blocks triggers like a key press.
    app.insert_resource(ModalActive(true));
    let gated = call(&mut app, "drive.trigger", json!({"name": "horn"})).expect_err("gated");
    assert_eq!(gated.code, code::GATED);
    // The listing shows the bound triggers with their aliases.
    app.insert_resource(ModalActive(false));
    let listing = call(&mut app, "drive.controls", json!({})).expect("controls");
    let triggers = listing["triggers"].as_array().expect("triggers");
    assert!(triggers
        .iter()
        .any(|t| t["name"] == "bus_doorfront0" && t["alias"] == "door_front"));
    assert!(triggers
        .iter()
        .any(|t| t["name"] == "horn" && t["held"] == true));
}

#[test]
fn camera_look_and_move_follow_the_mode_and_the_gates() {
    let mut app = drive_app();
    // Not in free roam: absolute angles and movement are refused with a reason.
    let e = call(&mut app, "camera.look", json!({"yaw": 45.0})).expect_err("yaw outside free roam");
    assert_eq!(e.code, code::NOT_READY);
    let e =
        call(&mut app, "camera.move", json!({"forward": 1.0})).expect_err("move outside free roam");
    assert_eq!(e.code, code::NOT_READY);
    let e = call(&mut app, "camera.look", json!({})).expect_err("nothing to do");
    assert_eq!(e.code, code::INVALID_PARAMS);
    // Free roam: yaw and pitch set the view directly (compass degrees, like `camera.teleport`).
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::FreeRoam;
    let look = call(
        &mut app,
        "camera.look",
        json!({"yaw": 90.0, "pitch": -10.0}),
    )
    .expect("look");
    assert!((look["yaw_deg"].as_f64().unwrap() - 90.0).abs() < 1e-3);
    assert!(
        (app.world().resource::<CameraState>().free_roam_pitch + 10f32.to_radians()).abs() < 1e-5
    );
    // Movement holds the movement keys and schedules their release.
    call(
        &mut app,
        "camera.move",
        json!({"forward": 1.0, "right": -1.0, "duration_ms": 200}),
    )
    .expect("move");
    let releases = app
        .world()
        .resource::<opts_bevy::control::state::ControlState>()
        .releases
        .len();
    assert_eq!(releases, 2, "W and A are released later");
    // A dialog blocks both, like the camera keys.
    app.insert_resource(ModalActive(true));
    let e = call(&mut app, "camera.move", json!({"forward": 1.0})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
    let e = call(&mut app, "camera.look", json!({"dx": 5.0})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
}

// ---- session vehicles ----------------------------------------------------------------------------

use opts_game::in_game::types::InGameEntity;
use opts_game::session_vehicles::{
    SessionVehicle, SessionVehicles, SessionVehiclesPlugin, SpawnParams, SpawnPoint,
};

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

#[test]
fn camera_viewpoint_get_set_save_reset() {
    let mut app = drive_app();
    let path = scratch("viewpoint").join("viewpoints.json");
    app.init_resource::<opts_bevy::ViewpointState>()
        .add_message::<opts_game::hud::RaiseAlert>()
        .insert_resource(opts_game::viewpoints::Viewpoints {
            path: Some(path.clone()),
            store: opts_game::viewpoints::ViewpointStore::default(),
        });

    // Outside the driver's view there is nothing to edit.
    let e = call(&mut app, "camera.viewpoint.set", json!({"x": 0.1})).expect_err("not cockpit");
    assert_eq!(e.code, code::NOT_READY);
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    let e = call(&mut app, "camera.viewpoint.set", json!({"x": 0.1})).expect_err("no identity");
    assert_eq!(e.code, code::NOT_READY);
    app.world_mut()
        .resource_mut::<opts_bevy::ViewpointState>()
        .vehicle_key = Some("man_sd77".to_owned());

    let got = call(&mut app, "camera.viewpoint.get", json!({})).expect("get");
    assert_eq!(got["editing"], false);
    assert_eq!(got["vehicle"], "man_sd77");
    assert_eq!(got["limits"]["offset_m"], 0.6);

    // Limits are refused, not clamped.
    for bad in [
        json!({"x": 1.0}),
        json!({"y": -0.7}),
        json!({"fov_deg": 10.0}),
        json!({"fov_deg": 130.0}),
        json!({}),
    ] {
        let e = call(&mut app, "camera.viewpoint.set", bad.clone()).expect_err("limit");
        assert_eq!(e.code, code::INVALID_PARAMS, "{bad}");
    }
    assert_eq!(
        call(&mut app, "camera.viewpoint.get", json!({})).expect("get")["editing"],
        false,
        "a refused set opens nothing"
    );

    let set = call(
        &mut app,
        "camera.viewpoint.set",
        json!({"x": 0.1, "y": -0.05, "fov_deg": 85.0}),
    )
    .expect("set");
    assert_eq!(set["editing"], true);
    assert_eq!(set["dirty"], true);
    assert!((set["viewpoint"]["x"].as_f64().unwrap() - 0.1).abs() < 1e-6);
    assert_eq!(set["viewpoint"]["fov_deg"], 85.0);
    assert!(!path.exists(), "nothing is stored before save");

    let saved = call(&mut app, "camera.viewpoint.save", json!({})).expect("save");
    assert_eq!(saved["editing"], false);
    assert_eq!(saved["dirty"], false);
    assert!((saved["stored"]["y"].as_f64().unwrap() + 0.05).abs() < 1e-6);
    let text = std::fs::read_to_string(&path).expect("file written before the answer");
    assert!(
        text.contains("man_sd77") && text.contains("\"version\": 1"),
        "{text}"
    );

    let reset = call(&mut app, "camera.viewpoint.reset", json!({})).expect("reset");
    assert_eq!(reset["viewpoint"]["x"], 0.0);
    assert!(reset["viewpoint"]["fov_deg"].is_null());
    let text = std::fs::read_to_string(&path).expect("file");
    assert!(!text.contains("man_sd77"), "the entry is gone: {text}");

    app.insert_resource(ModalActive(true));
    let e = call(&mut app, "camera.viewpoint.set", json!({"x": 0.1})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
}

#[test]
fn camera_reset_resets_the_current_view_and_follows_the_gates() {
    let mut app = drive_app();
    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.mode = CameraMode::External;
        s.external_yaw = 2.0;
        s.external_zoom = 33.0;
    }
    let r = call(&mut app, "camera.reset", json!({})).expect("reset external");
    assert_eq!(r["mode"], "external");
    let s = app.world().resource::<CameraState>();
    assert_eq!((s.external_yaw, s.external_zoom), (0.0, 8.0));

    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.mode = CameraMode::Cockpit;
        s.sub_camera_index = 2;
        s.view_yaw = 1.0;
    }
    let r = call(&mut app, "camera.reset", json!({})).expect("reset cockpit");
    assert_eq!((r["mode"].as_str(), r["sub_camera"].as_u64()), (Some("cockpit"), Some(2)));
    assert_eq!(app.world().resource::<CameraState>().view_yaw, 0.0);

    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.mode = CameraMode::FreeRoam;
        s.free_roam_speed = 80.0;
    }
    call(&mut app, "camera.reset", json!({})).expect("reset free");
    assert_eq!(
        app.world().resource::<CameraState>().free_roam_speed,
        opts_bevy::FREE_ROAM_MIN_SPEED
    );

    app.insert_resource(ModalActive(true));
    let e = call(&mut app, "camera.reset", json!({})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
}
