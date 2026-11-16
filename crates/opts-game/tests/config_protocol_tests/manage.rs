//! Integration tests for the config protocol: update, duplicate and delete, and damaged or newer files.
use super::*;

#[test]
fn update_duplicate_and_delete() {
    let mut f = fixture("manage");
    call(&mut f.app, "config.save", json!({ "name": "A" })).expect("saves");
    call(
        &mut f.app,
        "config.save",
        json!({ "name": "B", "replace": true }),
    )
    .expect("saves");
    let e = call(
        &mut f.app,
        "config.update",
        json!({ "id": "a", "name": "b" }),
    )
    .expect_err("taken");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let e = call(&mut f.app, "config.update", json!({ "id": "a" })).expect_err("nothing to do");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let u = call(
        &mut f.app,
        "config.update",
        json!({ "id": "a", "name": "Alpha", "notes": "n" }),
    )
    .expect("updates");
    assert_eq!(u["configuration"]["name"], "Alpha");
    assert_eq!(u["configuration"]["id"], "a");
    let d = call(
        &mut f.app,
        "config.duplicate",
        json!({ "id": "alpha", "name": "Copy" }),
    )
    .expect("copies");
    assert_eq!(d["id"], "copy");
    assert_eq!(d["configuration"]["notes"], "n");
    call(&mut f.app, "config.delete", json!({ "id": "copy" })).expect("deletes");
    let e = call(&mut f.app, "config.delete", json!({ "id": "copy" })).expect_err("gone");
    assert_eq!(e.code, code::NOT_FOUND);
    let e = call(&mut f.app, "config.load", json!({ "id": "nothing" })).expect_err("unknown");
    assert_eq!(e.code, code::NOT_FOUND);
    assert_eq!(
        call(&mut f.app, "config.list", json!({})).expect("lists")["count"],
        2
    );
}

#[test]
fn a_damaged_or_newer_file_is_reported_not_overwritten() {
    let mut f = fixture("file");
    let path = f.dir.join("bus_configurations.json");
    std::fs::create_dir_all(&f.dir).expect("dir");
    std::fs::write(&path, r#"{"version":99,"configurations":[]}"#).expect("writes");
    let e = call(&mut f.app, "config.list", json!({})).expect_err("newer");
    assert_eq!(e.code, code::FAILED);
    let e = call(&mut f.app, "config.save", json!({ "name": "x" })).expect_err("newer");
    assert_eq!(e.code, code::FAILED);
    assert!(std::fs::read_to_string(&path)
        .expect("reads")
        .contains("99"));
    std::fs::write(&path, "{ broken").expect("writes");
    let e = call(&mut f.app, "config.list", json!({})).expect_err("damaged");
    assert!(e.message.contains("damaged"), "{}", e.message);
    // A save moves the damaged file aside and starts a new one.
    BusConfigurations::default().save_to(&path).expect("saves");
    assert!(f.dir.join("bus_configurations.json.broken").exists());
}
