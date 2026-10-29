//! Unit tests for `super` (`share.rs`).

use std::path::PathBuf;

use opts_bevy::Viewpoint;

use super::*;

fn vp(x: f32) -> Viewpoint {
    Viewpoint {
        x,
        ..Viewpoint::default()
    }
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "opts_viewpoint_share_{name}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn store_with(vehicle: &str, index: usize, v: Viewpoint) -> ViewpointStore {
    let mut s = ViewpointStore::default();
    s.set(vehicle, index, v);
    s
}

#[test]
fn an_exported_file_is_read_back_unchanged() {
    let d = dir("roundtrip");
    let mut viewpoints = Viewpoints::default();
    viewpoints.store.set("man_sd77", 0, vp(0.05));
    viewpoints
        .store
        .set_model("man_sd77", "man_sd", 1, vp(-0.1));
    viewpoints.export_to(&d.join("shared.json")).unwrap();
    let (read, problems) = read_import(&d.join("shared.json")).unwrap();
    assert!(problems.is_empty());
    assert_eq!(read, viewpoints.store);
}

#[test]
fn an_import_adds_entries_and_the_imported_value_wins() {
    let mut mine = store_with("a", 0, vp(0.1));
    mine.set("b", 0, vp(0.2));
    let mut theirs = store_with("a", 0, vp(0.3));
    theirs.set("c", 1, vp(0.4));
    theirs.set_model("c", "m", 0, vp(0.5));
    let summary = mine.merge_from(&theirs);
    assert_eq!(
        (summary.vehicles, summary.models, summary.replaced),
        (2, 1, 1)
    );
    assert_eq!(mine.get("a", 0), Some(vp(0.3)));
    assert_eq!(
        mine.get("b", 0),
        Some(vp(0.2)),
        "an entry the file lacks stays"
    );
    assert_eq!(mine.get("c", 1), Some(vp(0.4)));
    assert_eq!(mine.models["m"][&0], vp(0.5));
}

#[test]
fn importing_an_equal_entry_replaces_nothing() {
    let mut mine = store_with("a", 0, vp(0.1));
    let summary = mine.merge_from(&store_with("a", 0, vp(0.1)));
    assert_eq!((summary.vehicles, summary.replaced), (1, 0));
}

#[test]
fn an_import_merges_into_what_the_file_holds_now_and_writes_it() {
    let d = dir("write");
    let path = d.join("viewpoints.json");
    store_with("other_run", 0, vp(0.07)).save_to(&path).unwrap();
    let mut viewpoints = Viewpoints {
        path: Some(path.clone()),
        store: ViewpointStore::default(),
    };
    let summary = viewpoints
        .import(&store_with("shared", 0, vp(0.2)))
        .unwrap();
    assert_eq!(summary.vehicles, 1);
    let on_disk = ViewpointStore::load_from(&path);
    assert_eq!(
        on_disk.get("other_run", 0),
        Some(vp(0.07)),
        "the other run's entry stays"
    );
    assert_eq!(on_disk.get("shared", 0), Some(vp(0.2)));
    assert_eq!(viewpoints.store, on_disk);
}

#[test]
fn a_file_that_cannot_be_used_is_refused_and_left_alone() {
    let d = dir("refuse");
    let junk = d.join("junk.json");
    std::fs::write(&junk, "this is not json").unwrap();
    assert!(read_import(&junk)
        .unwrap_err()
        .contains("not a usable viewpoints file"));
    assert!(junk.exists(), "an import never moves the user's file aside");
    let newer = d.join("newer.json");
    std::fs::write(&newer, r#"{"version": 99, "vehicles": {}}"#).unwrap();
    assert!(read_import(&newer)
        .unwrap_err()
        .contains("unsupported version"));
    assert!(read_import(&d.join("missing.json"))
        .unwrap_err()
        .contains("cannot read"));
}

#[test]
fn invalid_entries_are_dropped_with_a_reason_and_their_neighbours_come_in() {
    let d = dir("partial");
    let path = d.join("partial.json");
    std::fs::write(
        &path,
        r#"{"version": 1, "vehicles": {
            "good": {"cameras": {"0": {"x": 0.05, "y": 0.0, "z": 0.0, "fov_deg": null}}},
            "bad": {"cameras": {"0": {"x": 9.0, "y": 0.0, "z": 0.0, "fov_deg": null}}}}}"#,
    )
    .unwrap();
    let (store, problems) = read_import(&path).unwrap();
    assert!(store.get("good", 0).is_some() && store.get("bad", 0).is_none());
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("out of range"), "{problems:?}");
}
