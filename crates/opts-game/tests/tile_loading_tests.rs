//! The tile loading controls (M-0080) on a self-authored map of five tiles in a row: *load all*
//! keeps every tile resident however far the camera goes, a radius keeps the tiles around the
//! centre tile, and a changed setting is followed while the drive runs.

#[path = "tile_loading_tests/budget.rs"]
mod budget;
mod common;
#[path = "tile_loading_tests/modes.rs"]
mod modes;
#[path = "tile_loading_tests/protocol.rs"]
mod protocol;

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_game::{
    in_game::{
        omsi_world::WorldLoadJob, tile_loading::TileMode, InGamePlugin, SimulationLoadState,
    },
    new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem},
    settings::GameSettings,
    AppArgs, CliArgs,
};
use opts_gui::prelude::*;
use serde_json::{json, Value};

use common::{fresh_root, write_map, MiniMap};

const TILES: [(i32, i32); 5] = [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)];

fn catalogue() -> GameCatalogue {
    GameCatalogue {
        maps: vec![MapItem {
            id: "Row".into(),
            name: "Row".into(),
            friendly_name: "Row".into(),
            description: String::new(),
            tile_count: TILES.len(),
            standard_depot: String::new(),
            years: String::new(),
            entry_points: vec!["Mini start".into()],
        }],
        vehicles: vec![VehicleItem::named("Alpha_Bus", "Alpha", "Solo")],
    }
}

/// A drive without a vehicle (a free camera) on the five-tile map with the given settings.
fn drive(tag: &str, settings: &[(&str, &str)]) -> (App, std::path::PathBuf) {
    let root = fresh_root(tag);
    write_map(
        &root,
        &MiniMap {
            name: "Row",
            listed: TILES.to_vec(),
            written: TILES.to_vec(),
            dangling_objects: 0,
        },
    );
    let mut game_settings = GameSettings::default();
    for (key, value) in settings {
        game_settings.set_raw(key, value);
    }
    let setup = NewGameSetup {
        selected_map: "Row".into(),
        selected_entry_point: "Mini start".into(),
        no_vehicle: true,
        ..NewGameSetup::default()
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(game_settings)
        .insert_resource(MaterialTheme::dark())
        .init_state::<opts_game::app_state::AppState>()
        .add_plugins(OptsGuiPlugin)
        .insert_resource(catalogue())
        .insert_resource(setup)
        .insert_resource(AppArgs(CliArgs {
            headless: true,
            omsi_path: Some(root.clone()),
            ..CliArgs::default()
        }))
        .add_plugins(InGamePlugin)
        // The settings plugin's system that applies the two settings to the streamer.
        .add_systems(
            Update,
            opts_game::settings_registry::live::sync_tile_loading,
        );
    app.world_mut()
        .resource_mut::<NextState<opts_game::app_state::AppState>>()
        .set(opts_game::app_state::AppState::InGame);
    run(&mut app, 80);
    assert!(
        app.world().resource::<SimulationLoadState>().is_complete,
        "the drive loaded"
    );
    (app, root)
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn resident(app: &App) -> Vec<(i32, i32)> {
    app.world().resource::<WorldLoadJob>().resident_tiles()
}

fn mode(app: &App) -> TileMode {
    app.world().resource::<WorldLoadJob>().tile_mode()
}

/// Puts the free camera over the middle of `tile`.
fn move_camera_to(app: &mut App, tile: (i32, i32)) {
    let (x, y) = app.world().resource::<WorldLoadJob>().tile_offset(tile);
    let at = Transform::from_xyz(x + 150.0, 20.0, -(y + 150.0));
    let mut cameras = app
        .world_mut()
        .query_filtered::<(&mut Transform, &mut GlobalTransform), With<opts_bevy::MainCamera>>();
    for (mut t, mut g) in cameras.iter_mut(app.world_mut()) {
        *t = at;
        *g = at.into();
    }
}

/// `scene.tiles` through the dispatcher, with only that method registered.
fn scene_tiles(app: &mut App) -> Value {
    if app.world().get_resource::<ControlRegistry>().is_none() {
        let mut registry = ControlRegistry::default();
        opts_game::control_tiles::register(&mut registry);
        app.insert_resource(registry);
    }
    match dispatch(app.world_mut(), "scene.tiles", &json!({})) {
        Outcome::Done(r) => r.expect("scene.tiles answers"),
        Outcome::Wait(_) => panic!("scene.tiles does not wait"),
    }
}

fn set_budget(app: &mut App, budget: std::time::Duration) {
    app.world_mut().resource_mut::<WorldLoadJob>().stream_budget = budget;
}

fn building(app: &App) -> Option<(i32, i32)> {
    app.world().resource::<WorldLoadJob>().building_tile()
}
