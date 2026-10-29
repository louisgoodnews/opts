//! `camera.viewpoint.export` and `camera.viewpoint.import`: sharing viewpoints through a file.

use std::path::{Path, PathBuf};

use opts_bevy::{Viewpoint, ViewpointState};
use opts_game::viewpoints::{ViewpointStore, Viewpoints};

use super::*;

fn vp(x: f32) -> Viewpoint {
    Viewpoint {
        x,
        ..Viewpoint::default()
    }
}

/// An app with a viewpoint store in `file`, holding `store`; only `dir` may be read and written.
fn app_with(dir: &Path, file: &str, store: ViewpointStore) -> (App, PathBuf) {
    let mut app = base_app(vec![dir.to_path_buf()]);
    let file = dir.join(file);
    app.init_resource::<ViewpointState>()
        .insert_resource(Viewpoints {
            path: Some(file.clone()),
            store,
        });
    (app, file)
}

fn code_of(app: &mut App, method: &str, path: &Path) -> i64 {
    call(app, method, json!({"path": path.to_str().unwrap()}))
        .expect_err("refused")
        .code
}

#[test]
fn an_exported_file_can_be_imported_by_another_player() {
    let dir = scratch("viewpoint-share");
    let mut store = ViewpointStore::default();
    store.set("man_sd77", 0, vp(0.05));
    store.set_model("man_sd77", "man_sd", 1, vp(-0.1));
    let (mut mine, _) = app_with(&dir, "mine.json", store);
    let shared = dir.join("shared.json");
    let out = call(
        &mut mine,
        "camera.viewpoint.export",
        json!({"path": shared.to_str().unwrap()}),
    )
    .expect("export");
    assert_eq!(
        (out["vehicles"].as_u64(), out["models"].as_u64()),
        (Some(1), Some(1))
    );
    assert!(shared.is_file());

    let mut store = ViewpointStore::default();
    store.set("man_sd77", 0, vp(0.3));
    store.set("neoplan", 0, vp(0.2));
    let (mut theirs, theirs_file) = app_with(&dir, "theirs.json", store.clone());
    store.save_to(&theirs_file).expect("their file");
    theirs
        .world_mut()
        .resource_mut::<ViewpointState>()
        .loaded_for = Some(("neoplan".into(), 0));
    let out = call(
        &mut theirs,
        "camera.viewpoint.import",
        json!({"path": shared.to_str().unwrap()}),
    )
    .expect("import");
    assert_eq!(
        (out["vehicles"].as_u64(), out["models"].as_u64()),
        (Some(1), Some(1))
    );
    assert_eq!(out["replaced"], 1, "the shared value won for man_sd77");
    assert_eq!(out["problems"], json!([]));
    let on_disk = ViewpointStore::load_from(&theirs_file);
    assert_eq!(on_disk.get("man_sd77", 0), Some(vp(0.05)));
    assert_eq!(
        on_disk.get("neoplan", 0),
        Some(vp(0.2)),
        "their other entry stays"
    );
    assert_eq!(on_disk.models["man_sd"][&1], vp(-0.1));
    let state = theirs.world().resource::<ViewpointState>();
    assert!(
        state.loaded_for.is_none(),
        "the vehicle in the seat reloads its viewpoint"
    );
}

#[test]
fn files_that_cannot_be_used_are_refused() {
    let dir = scratch("viewpoint-share-refuse");
    let outside = scratch("viewpoint-share-outside");
    let (mut app, _) = app_with(&dir, "mine.json", ViewpointStore::default());
    let import = "camera.viewpoint.import";
    assert_eq!(
        code_of(&mut app, import, &dir.join("missing.json")),
        code::NOT_FOUND
    );
    let junk = dir.join("junk.json");
    std::fs::write(&junk, "not json").unwrap();
    assert_eq!(code_of(&mut app, import, &junk), code::INVALID_PARAMS);
    assert!(junk.is_file(), "an unusable file is left where it is");
    let elsewhere = outside.join("shared.json");
    std::fs::write(&elsewhere, ViewpointStore::default().to_json()).unwrap();
    assert_eq!(code_of(&mut app, import, &elsewhere), code::FORBIDDEN_PATH);
    let target = outside.join("export.json");
    assert_eq!(
        code_of(&mut app, "camera.viewpoint.export", &target),
        code::FORBIDDEN_PATH
    );
    let e = call(&mut app, import, json!({})).expect_err("path is required");
    assert_eq!(e.code, code::INVALID_PARAMS);
}

#[test]
fn an_import_waits_while_the_editor_is_open() {
    let dir = scratch("viewpoint-share-editing");
    let (mut app, _) = app_with(&dir, "mine.json", ViewpointStore::default());
    let shared = dir.join("shared.json");
    ViewpointStore::default().save_to(&shared).unwrap();
    app.world_mut().resource_mut::<ViewpointState>().editing = true;
    assert_eq!(
        code_of(&mut app, "camera.viewpoint.import", &shared),
        code::GATED
    );
}
