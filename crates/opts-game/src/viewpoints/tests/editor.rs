//! Editor tests of `viewpoints`: per-camera viewpoints, reset and open editors.

use super::*;

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
