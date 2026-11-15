//! Store tests: keys, the text round trip, corrupt and newer files, merging.


use super::*;


#[test]
fn keys_are_the_lower_case_bus_stem() {
    assert_eq!(
        key_of_bus_file(Path::new("/omsi/Vehicles/MAN_SD77/MAN_SD77.bus")),
        Some("man_sd77".to_owned())
    );
    assert_eq!(normalise_key("   "), None);
    assert_eq!(normalise_key("a\nb"), None);
}

#[test]
fn a_store_round_trips_through_its_text() {
    let mut s = ViewpointStore::default();
    s.set("bus_a", 0, vp(0.1, Some(80.0)));
    s.set("bus_a", 2, vp(-0.2, None));
    s.set("bus_b", 0, vp(0.0, None));
    let (back, report) = ViewpointStore::parse(&s.to_json());
    assert!(report.problems.is_empty(), "{report:?}");
    assert_eq!(back, s);
    assert_eq!(back.get("bus_a", 0), Some(vp(0.1, Some(80.0))));
    assert_eq!(back.get("bus_a", 1), None);
}

#[test]
fn the_default_viewpoint_removes_the_entry() {
    let mut s = ViewpointStore::default();
    s.set("bus_a", 0, vp(0.1, None));
    s.set("bus_a", 0, Viewpoint::default());
    assert!(s.vehicles.is_empty());
}

#[test]
fn corrupt_entries_are_dropped_and_the_rest_survives() {
    let text = r#"{
        "version": 1,
        "vehicles": {
            "good": { "cameras": { "0": { "x": 0.1, "y": 0.0, "z": 0.0 } } },
            "": { "cameras": {} },
            "far": { "cameras": { "0": { "x": 9.0 }, "1": { "fov_deg": 500 }, "x": { "x": 0.1 },
                                  "2": { "x": "no" }, "3": { "z": -0.2 } } },
            "nocams": 5
        }
    }"#;
    let (s, report) = ViewpointStore::parse(text);
    assert_eq!(s.get("good", 0).map(|v| v.x), Some(0.1));
    assert_eq!(s.get("far", 0), None, "out of range");
    assert_eq!(s.get("far", 1), None, "fov out of range");
    assert_eq!(s.get("far", 2), None, "wrong type");
    assert_eq!(
        s.get("far", 3).map(|v| v.z),
        Some(-0.2),
        "a neighbour survives"
    );
    assert!(report.problems.len() >= 5, "{report:?}");
    assert!(!report.unreadable);
}

#[test]
fn garbage_and_newer_files_give_an_empty_store() {
    let (s, r) = ViewpointStore::parse("this is not json");
    assert!(s.vehicles.is_empty() && r.unreadable);
    let (s, r) = ViewpointStore::parse(
        r#"{"version": 99, "vehicles": {"a": {"cameras": {"0": {"x": 0.1}}}}}"#,
    );
    assert!(s.vehicles.is_empty() && !r.unreadable && !r.problems.is_empty());
}

#[test]
fn an_unreadable_file_is_moved_aside() {
    let dir = temp_dir("bad");
    let path = dir.join("viewpoints.json");
    std::fs::write(&path, "{{{{").expect("write");
    let s = ViewpointStore::load_from(&path);
    assert!(s.vehicles.is_empty());
    assert!(!path.exists());
    assert!(dir.join("viewpoints.json.bad").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn saving_merges_with_what_another_run_wrote() {
    let dir = temp_dir("merge");
    let path = dir.join("viewpoints.json");
    let mut a = Viewpoints {
        path: Some(path.clone()),
        store: ViewpointStore::default(),
    };
    let mut b = a.clone();
    a.save("bus_a", None, 0, vp(0.1, None), false)
        .expect("save a");
    b.save("bus_b", None, 1, vp(0.2, None), false)
        .expect("save b");
    let all = ViewpointStore::load_from(&path);
    assert!(all.get("bus_a", 0).is_some() && all.get("bus_b", 1).is_some());
    let _ = std::fs::remove_dir_all(dir);
}
