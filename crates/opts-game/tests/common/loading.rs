//! Helpers of the loading-screen tests: a catalogue, an app with the loading screen and the
//! queries on what it shows.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    in_game::{
        load_report::{LoadOutcome, LoadReport},
        InGamePlugin, LoadingScreenRoot,
    },
    new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem},
    settings::GameSettings,
    AppArgs, CliArgs,
};
use opts_gui::prelude::*;
use opts_gui::ui_tree::find_entity;

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

/// Like [`loading_app`], with the catalogue and the choices given.
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

pub fn outcome(app: &App) -> LoadOutcome {
    app.world().resource::<LoadReport>().outcome
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

pub fn has(app: &mut App, id: &str) -> bool {
    find_entity(app.world_mut(), id).is_some()
}
