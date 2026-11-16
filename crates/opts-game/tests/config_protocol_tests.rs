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

#[path = "config_protocol_tests/manage.rs"]
mod manage;
#[path = "config_protocol_tests/save_load.rs"]
mod save_load;
#[path = "config_protocol_tests/session.rs"]
mod session;
