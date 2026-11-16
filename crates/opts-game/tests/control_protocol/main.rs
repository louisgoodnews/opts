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
use opts_cli::protocol::{code, RpcError};
use opts_game::app_state::AppState;
use opts_game::control::GameControlPlugin;
use opts_game::in_game::omsi_world::WorldStats;
use opts_sim::vehicle::inputs::GearPosition;
use serde_json::{json, Value};



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

mod commands;


mod drive;

mod observe;

mod triggers;

mod camera;
mod viewpoint_share;

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
