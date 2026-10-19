//! The loading screen on self-authored miniature installations: stages, the manifest of the real
//! session, seeded failures that must stay visible, and the three outcomes.

#[path = "../common/mod.rs"]
mod common;

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    in_game::{
        load_report::{LoadOutcome, LoadReport},
        CancelLoadingBtn, InGameEntity, InGameHudRoot, InGamePlugin, LoadStage, LoadingScreenRoot,
        SimulationLoadState,
    },
    new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem},
    settings::GameSettings,
    AppArgs, CliArgs,
};
use opts_gui::interaction::Activated;
use opts_gui::prelude::*;
use opts_gui::ui_tree::find_entity;

use common::{fresh_root, write_map, MiniMap};

fn catalogue(map: &str, vehicle: &str) -> GameCatalogue {
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
fn loading_app(map: &str, vehicle: &str, args: CliArgs) -> App {
    let setup = NewGameSetup {
        selected_map: map.into(),
        selected_entry_point: "Mini start".into(),
        selected_vehicle: vehicle.into(),
        ..NewGameSetup::default()
    };
    loading_app_with(catalogue(map, vehicle), setup, args)
}

/// Like [`loading_app`], with the catalogue and the choices given.
fn loading_app_with(catalogue: GameCatalogue, setup: NewGameSetup, args: CliArgs) -> App {
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

fn args_for(root: &std::path::Path) -> CliArgs {
    CliArgs {
        headless: true,
        omsi_path: Some(root.to_path_buf()),
        ..CliArgs::default()
    }
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn outcome(app: &App) -> LoadOutcome {
    app.world().resource::<LoadReport>().outcome
}

fn texts(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

fn screen_count(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<LoadingScreenRoot>>()
        .iter(app.world())
        .count()
}

fn has(app: &mut App, id: &str) -> bool {
    find_entity(app.world_mut(), id).is_some()
}

#[test]
fn the_screen_shows_stages_and_the_manifest_and_a_cancel_button() {
    let root = fresh_root("screen");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 2);
    assert_eq!(screen_count(&mut app), 1);
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "Preparing your drive"), "{t:?}");
    assert!(t.iter().any(|x| x == "CONTROLS"));
    assert!(
        !t.iter()
            .any(|x| x.contains("Start Driving") || x.contains("TRANSIT")),
        "no developer wording, no unusable button: {t:?}"
    );
    assert!(has(&mut app, "loading.cancel"));
    assert!(!has(&mut app, "loading.start"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_manifest_names_the_map_that_really_loads_not_the_one_the_menu_chose() {
    // F17: the menu says "Berlin-Spandau"; the command line points at "Mini". The card must
    // name what loads.
    let root = fresh_root("f17");
    let dir = write_map(&root, &MiniMap::simple("Mini"));
    let mut args = args_for(&root);
    args.map_path = Some(dir);
    let mut app = loading_app("Berlin-Spandau", "MiniBus", args);
    run(&mut app, 6);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x == "Mini"),
        "the loading map is named: {t:?}"
    );
    assert!(
        !t.iter().any(|x| x == "Berlin-Spandau"),
        "the menu's map must not be on the card: {t:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

mod problems;

#[test]
fn a_clean_load_continues_by_itself_without_a_button() {
    let mut app = loading_app(
        "Mini",
        "MiniBus",
        CliArgs {
            headless: true,
            ..CliArgs::default()
        },
    );
    // Skip the loader: the world is "done" and nothing went wrong.
    {
        let mut s = app.world_mut().resource_mut::<SimulationLoadState>();
        s.stage = LoadStage::Ready;
        s.is_complete = true;
    }
    {
        let mut r = app.world_mut().resource_mut::<LoadReport>();
        r.reset();
        r.finish();
    }
    assert_eq!(outcome(&app), LoadOutcome::Ready);
    run(&mut app, 6);
    assert_eq!(screen_count(&mut app), 0, "the screen continues by itself");
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    let hud = app
        .world_mut()
        .query_filtered::<Entity, With<InGameHudRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(hud, 1);
}

#[test]
fn cancel_asks_first_and_then_returns_to_plan_a_drive() {
    let root = fresh_root("cancel");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 2);
    let cancel = app
        .world_mut()
        .query_filtered::<Entity, With<CancelLoadingBtn>>()
        .iter(app.world())
        .next()
        .expect("cancel button");
    app.world_mut().write_message(Activated(cancel));
    run(&mut app, 3);
    assert!(
        has(&mut app, "loading.cancel-dialog"),
        "the dialog asks first"
    );
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame
    );
    let keep = find_entity(app.world_mut(), "loading.cancel-keep").expect("keep");
    app.world_mut().write_message(Activated(keep));
    run(&mut app, 3);
    assert!(!has(&mut app, "loading.cancel-dialog"));
    app.world_mut().write_message(Activated(cancel));
    run(&mut app, 3);
    let yes = find_entity(app.world_mut(), "loading.cancel-yes").expect("yes");
    app.world_mut().write_message(Activated(yes));
    run(&mut app, 4);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::VehicleSelection
    );
    let left = app
        .world_mut()
        .query_filtered::<Entity, With<InGameEntity>>()
        .iter(app.world())
        .count();
    assert_eq!(left, 0, "the session is torn down");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn retry_starts_the_load_again_and_clears_the_report() {
    let root = fresh_root("retry");
    std::fs::create_dir_all(root.join("maps")).expect("maps");
    let mut app = loading_app("Ghost Town", "MiniBus", args_for(&root));
    run(&mut app, 12);
    assert_eq!(outcome(&app), LoadOutcome::Failed);
    // The map appears; the retry finds it.
    write_map(&root, &MiniMap::simple("Ghost Town"));
    let retry = find_entity(app.world_mut(), "loading.retry").expect("retry");
    app.world_mut().write_message(Activated(retry));
    run(&mut app, 40);
    assert_ne!(
        outcome(&app),
        LoadOutcome::Failed,
        "{:?}",
        app.world().resource::<LoadReport>().problems
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_developer_overlay_is_hidden_while_the_loading_screen_is_up() {
    let root = fresh_root("overlay");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 40);
    assert_eq!(screen_count(&mut app), 1, "problems keep the screen up");
    let mut q = app.world_mut().query::<(
        &Visibility,
        &opts_game::in_game::omsi_world::DebugOverlayText,
    )>();
    let all: Vec<_> = q.iter(app.world()).map(|(v, _)| *v).collect();
    assert!(!all.is_empty(), "the overlay exists");
    assert!(all.iter().all(|v| *v == Visibility::Hidden), "{all:?}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_menu_camera_keeps_drawing_the_loading_screen_until_a_scene_camera_exists() {
    // Without a camera the window stays black: the failed outcome never gets a scene camera.
    let root = fresh_root("camera");
    std::fs::create_dir_all(root.join("maps")).expect("maps");
    let mut app = loading_app("Ghost Town", "MiniBus", args_for(&root));
    let cam = app
        .world_mut()
        .spawn((Camera2d, opts_game::MenuCamera))
        .id();
    run(&mut app, 20);
    assert_eq!(outcome(&app), LoadOutcome::Failed);
    let active = app.world().get::<Camera>(cam).map(|c| c.is_active);
    assert_eq!(
        active,
        Some(true),
        "the failed screen needs the menu camera"
    );
    // A scene camera appears: now the menu camera steps aside.
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Camera::default(),
    ));
    run(&mut app, 2);
    let active = app.world().get::<Camera>(cam).map(|c| c.is_active);
    assert_eq!(active, Some(false));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_loading_screen_meets_the_ci_rules_in_every_outcome() {
    use opts_gui::testing::rules;
    let root = fresh_root("rules");
    write_map(&root, &MiniMap::simple("Mini"));
    for failing in [false, true] {
        let map = if failing { "Ghost Town" } else { "Mini" };
        let mut app = loading_app(map, "MiniBus", args_for(&root));
        run(&mut app, 40);
        let want = if failing {
            LoadOutcome::Failed
        } else {
            LoadOutcome::ReadyWithProblems
        };
        assert_eq!(outcome(&app), want);
        for (name, found) in [
            ("targets", rules::small_targets(app.world_mut())),
            ("text", rules::small_text(app.world_mut())),
            ("disabled", rules::disabled_without_reason(app.world_mut())),
        ] {
            assert!(found.is_empty(), "{map} {name}: {}", found.join("\n"));
        }
        let t = rules::keyboard_traversal(&mut app);
        assert!(t.missed.is_empty(), "{map}: {:?}", t.missed);
    }
    let _ = std::fs::remove_dir_all(&root);
}

// ---- the vehicle line (M-0076) and the map-only start (M-0068) ------------------------------------

mod vehicles;
