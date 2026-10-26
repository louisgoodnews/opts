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
    opts_game::hardware::protocol::register(&mut app);
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
    assert_eq!(all["protocol_version"], "1.29.0");
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

mod drive;

mod observe;

mod triggers;

mod camera;

// ---- session vehicles ----------------------------------------------------------------------------

use opts_game::in_game::types::InGameEntity;
use opts_game::session_vehicles::{
    SessionVehicle, SessionVehicles, SessionVehiclesPlugin, SpawnParams, SpawnPoint,
};

mod session;

// ---- session save ------------------------------------------------------------------------------

use opts_game::in_game::types::{InGameSessionInfo, SimulationLoadState};
use opts_game::new_game_screen::types::NewGameSetup;
use opts_game::save::{SaveStorage, SaveStore};

mod save_load;
