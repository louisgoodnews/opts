//! Loading and updating configurations; damaged or newer files.


use super::*;


#[test]
pub(super) fn a_repaint_or_value_the_vehicle_no_longer_has_is_reported_and_left_out() {
    let mut f = fixture("partial");
    {
        let mut s = f.app.world_mut().resource_mut::<NewGameSetup>();
        s.custom_setvars.insert("display_ext".into(), 5);
        s.custom_setvars.insert("vis_x".into(), 0);
    }
    call(&mut f.app, "config.save", json!({ "name": "Mine" })).expect("saves");
    // The vehicle's repaints change: Blue is gone and display_ext only has 2.
    let mut changed = reps();
    changed.names = vec!["Red".into()];
    changed.setvars.remove("Blue");
    f.app.world_mut().resource_mut::<LiveryState>().repaints = Some(changed);
    let loaded = call(&mut f.app, "config.load", json!({ "id": "mine" })).expect("loads");
    let codes: Vec<&str> = loaded["problems"]
        .as_array()
        .expect("list")
        .iter()
        .filter_map(|p| p["code"].as_str())
        .collect();
    assert_eq!(codes, ["repaint_missing", "setvar_rejected"]);
    let s = setup(&f.app);
    assert_eq!(s.selected_livery, "");
    assert_eq!(s.custom_setvars.get("display_ext"), None);
    assert_eq!(s.custom_setvars.get("vis_x"), Some(&0));
}

#[test]
pub(super) fn update_duplicate_and_delete() {
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
pub(super) fn a_damaged_or_newer_file_is_reported_not_overwritten() {
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
