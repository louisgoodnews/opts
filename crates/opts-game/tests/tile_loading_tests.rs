//! The tile loading controls (M-0080) on a self-authored map of five tiles in a row: *load all*
//! keeps every tile resident however far the camera goes, a radius keeps the tiles around the
//! centre tile, and a changed setting is followed while the drive runs.

mod common;

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

#[test]
fn load_all_has_every_tile_resident_when_the_drive_starts_and_keeps_them() {
    let (mut app, root) = drive("tl-all", &[("gameplay.load_all_tiles", "true")]);
    assert_eq!(mode(&app), TileMode::All);
    assert_eq!(resident(&app), TILES.to_vec(), "all five after loading");
    move_camera_to(&mut app, (4, 0));
    run(&mut app, 30);
    assert_eq!(resident(&app), TILES.to_vec(), "none is unloaded far away");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_radius_keeps_only_the_tiles_around_the_centre_tile() {
    let (mut app, root) = drive("tl-radius", &[("gameplay.tile_radius", "1")]);
    assert_eq!(mode(&app), TileMode::Radius(1));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)], "3 x 3 around (0, 0)");
    // Far along the row: the tiles near the start leave (beyond radius + 1), those within the
    // radius of the new centre tile come in.
    move_camera_to(&mut app, (4, 0));
    run(&mut app, 30);
    assert_eq!(resident(&app), vec![(3, 0), (4, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_larger_radius_loads_more_tiles_while_loading() {
    let (app, root) = drive("tl-larger", &[("gameplay.tile_radius", "3")]);
    assert_eq!(mode(&app), TileMode::Radius(3));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn automatic_without_a_hardware_profile_is_the_old_3x3() {
    let (app, root) = drive("tl-auto", &[]);
    assert_eq!(mode(&app), TileMode::Radius(1));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_changed_setting_is_followed_during_the_drive() {
    let (mut app, root) = drive("tl-change", &[("gameplay.tile_radius", "1")]);
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)]);
    // Switching *load all* on loads the rest, one tile per frame.
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("gameplay.load_all_tiles", "true");
    run(&mut app, 30);
    assert_eq!(mode(&app), TileMode::All);
    assert_eq!(resident(&app), TILES.to_vec());
    // And off again: the camera is at the start, so the tiles beyond radius + 1 leave.
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("gameplay.load_all_tiles", "false");
    run(&mut app, 30);
    assert_eq!(mode(&app), TileMode::Radius(1));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0), (2, 0)]);
    let _ = std::fs::remove_dir_all(&root);
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

#[test]
fn scene_tiles_reports_the_mode_the_radius_and_the_resident_tiles() {
    let (mut app, root) = drive("tl-proto", &[("gameplay.tile_radius", "2")]);
    let r = scene_tiles(&mut app);
    assert_eq!(r["mode"], "radius");
    assert_eq!(r["radius"], 2);
    assert_eq!(r["automatic"], false);
    assert_eq!(r["setting_radius"], 2);
    assert_eq!(r["load_all"], false);
    assert_eq!(r["map_radius"], 4);
    assert_eq!(r["listed"], 5);
    assert_eq!(r["listed_missing"], 0);
    assert_eq!(r["start_tile"], json!([0, 0]));
    assert_eq!(r["resident_count"], 3);
    assert_eq!(r["resident"], json!([[0, 0], [1, 0], [2, 0]]));
    assert_eq!(r["queued"], 0);

    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("gameplay.load_all_tiles", "true");
    run(&mut app, 30);
    let r = scene_tiles(&mut app);
    assert_eq!(r["mode"], "all");
    assert_eq!(r["radius"], Value::Null);
    assert_eq!(r["load_all"], true);
    assert_eq!(r["resident_count"], 5);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn scene_tiles_says_not_ready_outside_a_drive() {
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_tiles::register(&mut registry);
    app.insert_resource(registry);
    match dispatch(app.world_mut(), "scene.tiles", &json!({})) {
        Outcome::Done(Err(e)) => assert_eq!(e.code, opts_cli::protocol::code::NOT_READY),
        _ => panic!("expected NOT_READY"),
    }
}
