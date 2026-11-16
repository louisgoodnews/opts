//! `vehicle.setvars` (M-0078) in the wizard: listing the options of the chosen vehicle with kind,
//! range, value and default, changing them all or none, and the refusals.

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_game::new_game_screen::livery::LiveryState;
use opts_game::new_game_screen::repaints::VehicleRepaints;
use opts_game::new_game_screen::types::NewGameSetup;
use serde_json::{json, Value};

fn reps() -> VehicleRepaints {
    let mut reps = VehicleRepaints {
        default_name: "Red".into(),
        folder: "Citybus 530 by Kajosoft".into(),
        ..VehicleRepaints::default()
    };
    reps.setvars.insert(
        "Red".into(),
        vec![
            ("vis_x".into(), "1".into()),
            ("display_ext".into(), "2".into()),
            ("vehicle_id".into(), "3964".into()),
            ("language_ibis".into(), "1".into()),
        ],
    );
    reps.setvars
        .insert("Blue".into(), vec![("display_ext".into(), "5".into())]);
    reps
}

fn app(with_repaints: bool) -> App {
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_setvars::register(&mut registry);
    app.insert_resource(registry)
        .insert_resource(NewGameSetup::default());
    let mut liveries = LiveryState::default();
    if with_repaints {
        liveries.repaints = Some(reps());
    }
    app.insert_resource(liveries);
    app
}

fn call(app: &mut App, params: Value) -> Result<Value, RpcError> {
    match dispatch(app.world_mut(), "vehicle.setvars", &params) {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

fn option<'a>(list: &'a Value, name: &str) -> &'a Value {
    list["options"]
        .as_array()
        .and_then(|o| o.iter().find(|x| x["name"] == name))
        .unwrap_or_else(|| panic!("no option {name}"))
}

fn custom(app: &App, name: &str) -> Option<i32> {
    app.world()
        .resource::<NewGameSetup>()
        .custom_setvars
        .get(name)
        .copied()
}

#[path = "setvars_protocol_tests/batch.rs"]
mod batch;
#[path = "setvars_protocol_tests/listing.rs"]
mod listing;
