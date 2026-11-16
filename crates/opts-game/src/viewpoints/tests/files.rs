//! File tests of `viewpoints`: unreadable files and saves that merge.

use super::*;

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
