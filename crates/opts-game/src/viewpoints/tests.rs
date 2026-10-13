use super::*;

fn vp(x: f32, fov: Option<f32>) -> Viewpoint {
    Viewpoint {
        x,
        y: 0.05,
        z: -0.1,
        fov_deg: fov,
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/viewpoint-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

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
    assert_eq!(s.get("far", 3).map(|v| v.z), Some(-0.2), "a neighbour survives");
    assert!(report.problems.len() >= 5, "{report:?}");
    assert!(!report.unreadable);
}

#[test]
fn garbage_and_newer_files_give_an_empty_store() {
    let (s, r) = ViewpointStore::parse("this is not json");
    assert!(s.vehicles.is_empty() && r.unreadable);
    let (s, r) = ViewpointStore::parse(r#"{"version": 99, "vehicles": {"a": {"cameras": {"0": {"x": 0.1}}}}}"#);
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
    a.save("bus_a", 0, vp(0.1, None)).expect("save a");
    b.save("bus_b", 1, vp(0.2, None)).expect("save b");
    let all = ViewpointStore::load_from(&path);
    assert!(all.get("bus_a", 0).is_some() && all.get("bus_b", 1).is_some());
    let _ = std::fs::remove_dir_all(dir);
}

// ---- the systems: edit -> save -> respawn -> loaded ------------------------------------------

fn app(path: PathBuf) -> App {
    let mut app = App::new();
    app.init_resource::<CameraState>()
        .init_resource::<ViewpointState>()
        .insert_resource(Viewpoints {
            path: Some(path),
            store: ViewpointStore::default(),
        })
        .add_message::<RaiseAlert>()
        .add_systems(Update, (load_viewpoint, persist_viewpoint).chain());
    app
}

fn spawn_vehicle(app: &mut App, key: &str) -> Entity {
    app.world_mut()
        .spawn((PlayerVehicle, ViewpointKey(key.to_owned())))
        .id()
}

fn edit_and_save(app: &mut App, change: impl FnOnce(&mut ViewpointState)) {
    let mut st = app.world_mut().resource_mut::<ViewpointState>();
    st.open();
    change(&mut st);
    st.save();
}

#[test]
fn edit_save_respawn_loaded() {
    let dir = temp_dir("respawn");
    let path = dir.join("viewpoints.json");
    let mut app = app(path.clone());
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    let first = spawn_vehicle(&mut app, "man_sd77");
    app.world_mut().resource_mut::<ViewpointState>().default_fov_deg = 70.0;
    app.update();
    assert!(app.world().resource::<ViewpointState>().active.is_default());

    edit_and_save(&mut app, |s| {
        s.adjust(opts_bevy::ViewpointRow::Y, -5.0);
        s.adjust(opts_bevy::ViewpointRow::Fov, 10.0);
    });
    app.update();
    assert!(path.exists(), "the file is written");

    // Respawn: a new run with a fresh state reads the file.
    app.world_mut().despawn(first);
    app.update();
    let mut fresh = self::app(path.clone());
    fresh.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    fresh.world_mut().resource_mut::<ViewpointState>().default_fov_deg = 70.0;
    spawn_vehicle(&mut fresh, "man_sd77");
    fresh.update();
    let st = fresh.world().resource::<ViewpointState>();
    assert_eq!(st.active.y, -0.05);
    assert_eq!(st.active.fov_deg, Some(80.0));
    assert_eq!(st.active, st.committed);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn vehicles_do_not_share_viewpoints() {
    let dir = temp_dir("isolation");
    let path = dir.join("viewpoints.json");
    let mut a = app(path.clone());
    a.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    spawn_vehicle(&mut a, "bus_one");
    a.update();
    edit_and_save(&mut a, |s| s.adjust(opts_bevy::ViewpointRow::X, 7.0));
    a.update();

    let mut b = app(path.clone());
    b.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    spawn_vehicle(&mut b, "bus_two");
    b.update();
    assert!(b.world().resource::<ViewpointState>().active.is_default());
    edit_and_save(&mut b, |s| s.adjust(opts_bevy::ViewpointRow::X, -3.0));
    b.update();

    let all = ViewpointStore::load_from(&path);
    assert_eq!(all.get("bus_one", 0).map(|v| v.x), Some(0.07));
    assert_eq!(all.get("bus_two", 0).map(|v| v.x), Some(-0.03));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn each_driver_camera_has_its_own_viewpoint() {
    let dir = temp_dir("cameras");
    let path = dir.join("viewpoints.json");
    let mut a = app(path);
    a.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    spawn_vehicle(&mut a, "bus_one");
    a.update();
    edit_and_save(&mut a, |s| s.adjust(opts_bevy::ViewpointRow::Z, 4.0));
    a.update();
    a.world_mut().resource_mut::<CameraState>().sub_camera_index = 1;
    a.update();
    assert!(a.world().resource::<ViewpointState>().active.is_default());
    a.world_mut().resource_mut::<CameraState>().sub_camera_index = 0;
    a.update();
    assert_eq!(a.world().resource::<ViewpointState>().active.z, 0.04);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn reset_removes_the_stored_entry() {
    let dir = temp_dir("reset");
    let path = dir.join("viewpoints.json");
    let mut a = app(path.clone());
    a.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    spawn_vehicle(&mut a, "bus_one");
    a.update();
    edit_and_save(&mut a, |s| s.adjust(opts_bevy::ViewpointRow::X, 4.0));
    a.update();
    assert!(ViewpointStore::load_from(&path).get("bus_one", 0).is_some());
    edit_and_save(&mut a, ViewpointState::reset);
    a.update();
    assert!(ViewpointStore::load_from(&path).get("bus_one", 0).is_none());
    assert!(a.world().resource::<ViewpointState>().active.is_default());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_open_editor_is_not_overwritten_by_the_loader() {
    let dir = temp_dir("editing");
    let mut a = app(dir.join("viewpoints.json"));
    a.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    spawn_vehicle(&mut a, "bus_one");
    a.update();
    {
        let mut st = a.world_mut().resource_mut::<ViewpointState>();
        st.open();
        st.adjust(opts_bevy::ViewpointRow::X, 2.0);
    }
    a.update();
    assert_eq!(a.world().resource::<ViewpointState>().active.x, 0.02);
    let _ = std::fs::remove_dir_all(dir);
}
