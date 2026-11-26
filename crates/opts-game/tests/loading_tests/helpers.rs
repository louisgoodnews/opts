//! The loading-screen app of a named map and vehicle, and the queries on the loading outcome and
//! on the nodes of the loading screen. Used by the loading-screen tests only.

use bevy::prelude::*;
use opts_game::{
    in_game::load_report::{LoadOutcome, LoadReport},
    new_game_screen::types::NewGameSetup,
    CliArgs,
};
use opts_gui::ui_tree::find_entity;

use crate::loading::{catalogue, loading_app_with};

/// An app with the in-game plugin, a catalogue naming `map` and `vehicle`, and `args`.
pub fn loading_app(map: &str, vehicle: &str, args: CliArgs) -> App {
    let setup = NewGameSetup {
        selected_map: map.into(),
        selected_entry_point: "Mini start".into(),
        selected_vehicle: vehicle.into(),
        ..NewGameSetup::default()
    };
    loading_app_with(catalogue(map, vehicle), setup, args)
}

pub fn outcome(app: &App) -> LoadOutcome {
    app.world().resource::<LoadReport>().outcome
}

pub fn has(app: &mut App, id: &str) -> bool {
    find_entity(app.world_mut(), id).is_some()
}
