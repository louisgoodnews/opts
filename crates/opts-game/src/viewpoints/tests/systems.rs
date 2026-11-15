//! Systems tests: edit, save, respawn and load of viewpoints.


use super::*;


#[test]
fn edit_save_respawn_loaded() {
    let dir = temp_dir("respawn");
    let path = dir.join("viewpoints.json");
    let mut app = app(path.clone());
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    let first = spawn_vehicle(&mut app, "man_sd77");
    app.world_mut()
        .resource_mut::<ViewpointState>()
        .default_fov_deg = 70.0;
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
    fresh
        .world_mut()
        .resource_mut::<ViewpointState>()
        .default_fov_deg = 70.0;
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
