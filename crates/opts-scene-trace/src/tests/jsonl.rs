//! The JSON Lines file: header, records, the size cap and the relative paths.

use super::*;

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
