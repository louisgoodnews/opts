//! Helpers of the loading-screen tests: a catalogue, an app with the loading screen and the
//! queries on what it shows.

use bevy::prelude::*;
use opts_game::{
    in_game::{InGamePlugin, LoadingScreenRoot},
    new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem},
    settings::GameSettings,
    AppArgs, CliArgs,
};
use opts_gui::prelude::*;

pub fn catalogue(map: &str, vehicle: &str) -> GameCatalogue {
    GameCatalogue {
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
        vehicles: vec![VehicleItem::named(vehicle, "", vehicle)],
    }
}

/// An app with the in-game plugin, the catalogue and the choices given, and `args`.
pub fn loading_app_with(catalogue: GameCatalogue, setup: NewGameSetup, args: CliArgs) -> App {
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

pub fn args_for(root: &std::path::Path) -> CliArgs {
    CliArgs {
        headless: true,
        omsi_path: Some(root.to_path_buf()),
        ..CliArgs::default()
    }
}

pub fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

pub fn texts(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

pub fn screen_count(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<LoadingScreenRoot>>()
        .iter(app.world())
        .count()
}

