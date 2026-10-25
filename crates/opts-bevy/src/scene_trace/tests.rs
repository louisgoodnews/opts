//! Unit tests of `scene_trace`.

use std::path::Path;
use std::sync::Mutex;

use bevy::math::Vec3;

use super::*;

/// The trace is process-global state: tests that touch it run one at a time.
static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "opts-scene-trace-test-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn rec(kind: &str, tile: (i32, i32), file: &str, at: [f32; 3]) -> (TraceRecord, [f32; 3]) {
    (
        TraceRecord {
            kind: kind.into(),
            tile: Some([tile.0, tile.1]),
            file: file.into(),
            world: Some(TraceTransform {
                t: at,
                q: [0.0, 0.0, 0.0, 1.0],
                s: [1.0; 3],
            }),
            bounds: Some(TraceBounds {
                min: [-1.0; 3],
                max: [1.0; 3],
            }),
            ..TraceRecord::default()
        },
        at,
    )
}

fn put(kind: &str, tile: (i32, i32), file: &str, at: [f32; 3]) -> Option<u64> {
    let (r, w) = rec(kind, tile, file, at);
    record(
        &Probe {
            kind,
            tile: Some(tile),
            file,
            world: w,
        },
        || r,
    )
}

#[test]
fn off_allocates_nothing() {
    let _g = guard();
    set_mode(Mode::Off);
    assert!(!enabled());
    let id = record(
        &Probe {
            kind: kind::SCENERY,
            tile: Some((0, 0)),
            file: "a.sco",
            world: [0.0; 3],
        },
        || panic!("the record must not be built while the trace is off"),
    );
    assert_eq!(id, None);
    note_mesh(Path::new("x.o3d"), MeshNote::default());
    begin_tile((0, 0));
    end_tile((0, 0));
    assert!(!state_allocated(), "no state while off");
    assert!(!Lap::start().active());
    assert_eq!(status().mode, "");
}

#[test]
fn full_mode_keeps_every_record_and_summary_only_issues() {
    let _g = guard();
    set_directory(scratch("modes"));
    set_mode(Mode::Full);
    begin_session("Testmap", Path::new("/omsi"));
    put(kind::SCENERY, (1, 1), "Sceneryobjects/a.sco", [0.0; 3]).unwrap();
    put(kind::TREE, (1, 1), "Sceneryobjects/t.sco", [5.0, 0.0, 0.0]).unwrap();
    assert_eq!(status().in_memory, 2);
    set_mode(Mode::Off);

    set_mode(Mode::Summary);
    begin_session("Testmap", Path::new("/omsi"));
    put(kind::SCENERY, (1, 1), "a.sco", [0.0; 3]).unwrap();
    let (mut bad, w) = rec(kind::SCENERY, (1, 1), "b.sco", [0.0; 3]);
    bad.warnings.push("o3d not found".into());
    record(
        &Probe {
            kind: kind::SCENERY,
            tile: Some((1, 1)),
            file: "b.sco",
            world: w,
        },
        || bad,
    )
    .unwrap();
    let s = status();
    assert_eq!(s.records, 2);
    assert_eq!(
        s.in_memory, 1,
        "summary keeps only the record with an issue"
    );
    let t = tile_summary((1, 1)).unwrap();
    assert_eq!((t.total, t.issue_total), (2, 1));
    set_mode(Mode::Off);
}

#[test]
fn filters_by_tile_kind_file_and_radius() {
    let _g = guard();
    set_directory(scratch("filters"));
    set_mode(Mode::Full);
    begin_session("Testmap", Path::new("/omsi"));
    set_filters(Filters {
        tiles: vec![(2, 2)],
        ..Filters::default()
    });
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [0.0; 3]).is_none());
    assert!(put(kind::SCENERY, (2, 2), "a.sco", [0.0; 3]).is_some());

    set_filters(Filters {
        kinds: vec![kind::TREE.into()],
        ..Filters::default()
    });
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [0.0; 3]).is_none());
    assert!(put(kind::TREE, (1, 1), "a.sco", [0.0; 3]).is_some());

    set_filters(Filters {
        file: Some("Haus*.SCO".into()),
        ..Filters::default()
    });
    assert!(put(
        kind::SCENERY,
        (1, 1),
        "Sceneryobjects/x/haus_01.sco",
        [0.0; 3]
    )
    .is_some());
    assert!(put(kind::SCENERY, (1, 1), "Sceneryobjects/x/baum.sco", [0.0; 3]).is_none());

    set_filters(Filters {
        radius: Some(Radius {
            center: [100.0, 100.0],
            meters: 10.0,
        }),
        ..Filters::default()
    });
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [105.0, 0.0, 100.0]).is_some());
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [0.0, 0.0, 0.0]).is_none());
    set_mode(Mode::Off);
}

#[test]
fn jsonl_has_header_records_and_parses_back() {
    let _g = guard();
    let dir = scratch("jsonl");
    set_directory(dir.clone());
    set_mode(Mode::Full);
    begin_session("Test Map", Path::new("/omsi"));
    let id = put(
        kind::SCENERY,
        (3, 4),
        "Sceneryobjects/a.sco",
        [1.0, 2.0, 3.0],
    )
    .unwrap();
    end_tile((3, 4));
    let file = PathBuf::from(status().file.unwrap());
    set_mode(Mode::Off);
    let text = std::fs::read_to_string(&file).unwrap();
    let lines: Vec<serde_json::Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[0]["type"], "header");
    assert_eq!(lines[0]["schema"], 1);
    assert_eq!(lines[0]["map"], "Test Map");
    assert_eq!(lines[0]["mode"], "full");
    assert!(lines[0]["build"]["version"].is_string());
    let r = lines.iter().find(|l| l["type"] == "record").unwrap();
    let back: TraceRecord = serde_json::from_value(r["record"].clone()).unwrap();
    assert_eq!(back.id, id);
    assert_eq!(back.kind, "scenery");
    assert_eq!(back.tile, Some([3, 4]));
    assert!(lines.iter().any(|l| l["type"] == "tile_summary"));
    assert_eq!(lines.last().unwrap()["type"], "footer");
    assert!(file
        .file_name()
        .unwrap()
        .to_string_lossy()
        .ends_with("Test_Map.jsonl"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn size_cap_truncates_the_file_but_not_the_index() {
    let _g = guard();
    let dir = scratch("cap");
    set_directory(dir.clone());
    set_mode(Mode::Full);
    begin_session("Cap", Path::new("/omsi"));
    set_size_cap(600);
    for i in 0..20 {
        put(kind::SCENERY, (0, 0), "a.sco", [i as f32, 0.0, 0.0]).unwrap();
    }
    let s = status();
    assert!(s.truncated);
    assert_eq!(s.in_memory, 20);
    let file = PathBuf::from(s.file.unwrap());
    set_mode(Mode::Off);
    let size = std::fs::metadata(&file).unwrap().len();
    assert!(size < 1500, "file stays near the cap, was {size}");
    assert!(std::fs::read_to_string(&file)
        .unwrap()
        .contains("\"truncated\""));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn paths_are_relative_to_the_omsi_folder() {
    let _g = guard();
    set_directory(scratch("paths"));
    set_mode(Mode::Summary);
    begin_session("P", Path::new("/home/someone/OMSI 2"));
    assert_eq!(
        relative(Path::new(
            "/home/someone/OMSI 2/Sceneryobjects/a/model/x.o3d"
        )),
        "Sceneryobjects/a/model/x.o3d"
    );
    // An absolute path outside the installation on every platform (`/elsewhere` has no drive
    // letter, so it is not absolute on Windows).
    let outside = std::env::temp_dir().join("elsewhere/secret/x.o3d");
    assert_eq!(relative(&outside), "x.o3d");
    set_mode(Mode::Off);
}

#[test]
fn query_by_file_stage_and_tile_and_get() {
    let _g = guard();
    set_directory(scratch("query"));
    set_mode(Mode::Full);
    begin_session("Q", Path::new("/omsi"));
    let (mut r, w) = rec(
        kind::SCENERY,
        (1, 2),
        "Sceneryobjects/h/house.sco",
        [0.0; 3],
    );
    r.meshes.push(MeshNote {
        file: "Sceneryobjects/h/model/house.o3d".into(),
        stage: "decode-cache".into(),
        ..MeshNote::default()
    });
    let id = record(
        &Probe {
            kind: kind::SCENERY,
            tile: Some((1, 2)),
            file: "house.sco",
            world: w,
        },
        || r,
    )
    .unwrap();
    put(kind::SCENERY, (1, 2), "other.sco", [0.0; 3]).unwrap();
    let by_mesh = query(&Query {
        file: Some("model/house.o3d".into()),
        ..Query::default()
    });
    assert_eq!(by_mesh.len(), 1);
    let by_stage = query(&Query {
        stage: Some("decode-cache".into()),
        tile: Some((1, 2)),
        ..Query::default()
    });
    assert_eq!(by_stage.len(), 1);
    assert_eq!(get(id).unwrap().file, "Sceneryobjects/h/house.sco");
    begin_tile((1, 2));
    assert!(
        query(&Query::default()).is_empty(),
        "a rebuilt tile supersedes its records"
    );
    set_mode(Mode::Off);
}

#[test]
fn pick_returns_the_nearest_box_on_the_ray() {
    let (near, _) = rec(kind::SCENERY, (0, 0), "near", [0.0, 0.0, -10.0]);
    let (far, _) = rec(kind::SCENERY, (0, 0), "far", [0.0, 0.0, -30.0]);
    let (aside, _) = rec(kind::SCENERY, (0, 0), "aside", [20.0, 0.0, -10.0]);
    let mut recs = vec![near, far, aside];
    for (i, r) in recs.iter_mut().enumerate() {
        r.id = i as u64 + 1;
    }
    let hits = pick::pick(&recs, Vec3::ZERO, Vec3::NEG_Z);
    assert_eq!(hits.iter().map(|h| h.id).collect::<Vec<_>>(), vec![1, 2]);
    assert!((hits[0].distance - 9.0).abs() < 1e-4);
    // A rotated, scaled box.
    let mut r = recs[0].clone();
    let w = r.world.as_mut().unwrap();
    w.q = bevy::math::Quat::from_rotation_y(0.5).to_array();
    w.s = [2.0, 1.0, 1.0];
    assert!(pick::ray_hits(&r, Vec3::ZERO, Vec3::NEG_Z).is_some());
    assert!(pick::ray_hits(&r, Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Z).is_none());
}

#[test]
fn mode_parsing() {
    assert_eq!(Mode::parse("Full"), Some(Mode::Full));
    assert_eq!(Mode::parse("summary"), Some(Mode::Summary));
    assert_eq!(Mode::parse(""), Some(Mode::Off));
    assert_eq!(Mode::parse("loud"), None);
}

#[test]
fn glob_patterns() {
    use filter::glob_match;
    assert!(glob_match("house", "Sceneryobjects/House_01.sco"));
    assert!(glob_match("*.sco", "a/b.SCO"));
    assert!(glob_match(
        "sceneryobjects/*/house*.sco",
        "Sceneryobjects/x/House_01.sco"
    ));
    assert!(!glob_match("*.o3d", "a.sco"));
}
