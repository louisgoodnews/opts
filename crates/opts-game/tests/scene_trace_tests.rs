//! The scene trace on a self-authored miniature installation: a record for every placed object
//! (here: objects whose files are missing, which must show up as failed records with the reason),
//! the JSONL file, and no state at all while the trace is off. The trace is process-global, so
//! everything lives in one test.

mod common;
#[path = "common/simple_map.rs"]
mod simple_map;

use bevy::prelude::*;
use opts_bevy::scene_trace as st;
use opts_game::{
    app_state::AppState,
    in_game::InGamePlugin,
    new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem},
    settings::GameSettings,
    AppArgs, CliArgs,
};
use opts_gui::prelude::*;

fn app_for(root: &std::path::Path, map: &str) -> App {
    let catalogue = GameCatalogue {
        maps: vec![MapItem {
            id: map.into(),
            name: map.into(),
            friendly_name: map.into(),
            description: String::new(),
            tile_count: 1,
            standard_depot: String::new(),
            years: String::new(),
            entry_points: vec!["Mini start".into()],
        }],
        vehicles: vec![VehicleItem::named("MiniBus", "", "MiniBus")],
    };
    let setup = NewGameSetup {
        selected_map: map.into(),
        selected_entry_point: "Mini start".into(),
        selected_vehicle: "MiniBus".into(),
        ..NewGameSetup::default()
    };
    let args = CliArgs {
        headless: true,
        omsi_path: Some(root.to_path_buf()),
        ..CliArgs::default()
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .insert_resource(catalogue)
        .insert_resource(setup)
        .insert_resource(AppArgs(args))
        .add_plugins(InGamePlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    app
}

mod records;
