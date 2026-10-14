//! The scene trace on a self-authored miniature installation: a record for every placed object
//! (here: objects whose files are missing, which must show up as failed records with the reason),
//! the JSONL file, and no state at all while the trace is off. The trace is process-global, so
//! everything lives in one test.

mod common;

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

use common::{fresh_root, write_map, MiniMap};

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

#[test]
fn every_placed_object_has_a_record_and_off_leaves_no_state() {
    // Off: loading a map leaves no trace state behind.
    st::set_mode(st::Mode::Off);
    let root = fresh_root("trace_off");
    let mut map = MiniMap::simple("Mini");
    map.dangling_objects = 3;
    write_map(&root, &map);
    let mut app = app_for(&root, "Mini");
    for _ in 0..30 {
        app.update();
    }
    assert!(!st::state_allocated(), "no trace state while off");
    drop(app);

    // Full: the entry point object and the three dangling objects each have a record.
    let logs = std::env::temp_dir().join(format!("opts_trace_logs_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&logs);
    st::set_directory(logs.clone());
    st::set_mode(st::Mode::Full);
    let mut app = app_for(&root, "Mini");
    for _ in 0..30 {
        app.update();
    }
    let objects = st::query(&st::Query {
        kind: Some(st::kind::SCENERY.into()),
        limit: 100,
        ..st::Query::default()
    });
    assert_eq!(
        objects.len(),
        4,
        "one record per placed object: {objects:#?}"
    );
    for r in &objects {
        assert_eq!(r.tile, Some([0, 0]));
        assert_eq!(r.list.as_deref(), Some("objects"));
        assert_eq!(r.result, "failed", "{r:?}");
        assert!(
            r.warnings.iter().any(|w| w.contains("not found")),
            "the reason is recorded: {r:?}"
        );
        assert!(r.pos.is_some() && r.world.is_some() && r.rot_deg.is_some());
        assert!(r.is_issue());
    }
    let files: Vec<&str> = objects.iter().map(|r| r.file.as_str()).collect();
    assert!(
        files.iter().any(|f| f.contains("missing_2.sco")),
        "{files:?}"
    );
    let indexes: Vec<usize> = {
        let mut v: Vec<usize> = objects.iter().filter_map(|r| r.index).collect();
        v.sort_unstable();
        v
    };
    assert_eq!(
        indexes,
        vec![0, 1, 2, 3],
        "record indexes follow the .map order"
    );
    // The tile has a terrain record and a summary that counts the objects.
    let tile = st::tile_summary((0, 0)).expect("tile summary");
    assert_eq!(tile.kinds.get("scenery"), Some(&4));
    assert_eq!(tile.issues.get("scenery"), Some(&4));
    // The run file exists next to the logs, starts with a header and names the map.
    let status = st::status();
    let file = std::path::PathBuf::from(status.file.expect("trace file"));
    assert!(file.starts_with(&logs));
    drop(app);
    st::set_mode(st::Mode::Off);
    let text = std::fs::read_to_string(&file).unwrap();
    let first: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(first["type"], "header");
    assert!(first["map"].as_str().unwrap().contains("Mini"));
    assert!(
        text.lines()
            .filter(|l| l.contains("\"type\":\"record\""))
            .count()
            >= 4
    );
    assert!(
        !text.contains(root.to_string_lossy().as_ref()),
        "no absolute paths in the file"
    );
    let _ = std::fs::remove_dir_all(&logs);
    let _ = std::fs::remove_dir_all(&root);
}
