//! Tests of model-wide viewpoints and the default field of view.


use super::*;


// ---- model-wide viewpoints and the default FOV ------------------------------------------------

#[test]
fn model_entries_apply_to_every_vehicle_of_the_model_and_a_vehicle_entry_wins() {
    let mut s = ViewpointStore::default();
    s.set_model("sd77", "man_sd200", 0, vp(0.1, None));
    assert_eq!(s.lookup("sd77", Some("man_sd200"), 0), Some(vp(0.1, None)));
    assert_eq!(s.lookup("sd80", Some("man_sd200"), 0), Some(vp(0.1, None)));
    assert_eq!(
        s.lookup("sd80", Some("other"), 0),
        None,
        "another model is untouched"
    );
    assert_eq!(s.lookup("sd80", None, 0), None);

    // sd80 gets its own view: it wins over the model's.
    s.set_vehicle("sd80", Some("man_sd200"), 0, vp(-0.2, None));
    assert_eq!(s.lookup("sd80", Some("man_sd200"), 0), Some(vp(-0.2, None)));
    assert_eq!(s.lookup("sd77", Some("man_sd200"), 0), Some(vp(0.1, None)));

    // "Vehicle default" on sd80 must not fall back to the model: an explicit default is kept.
    s.set_vehicle("sd80", Some("man_sd200"), 0, Viewpoint::default());
    assert_eq!(
        s.lookup("sd80", Some("man_sd200"), 0),
        Some(Viewpoint::default())
    );

    // Saving for the model again drops sd80's own entry, so it shows the model's viewpoint.
    s.set_model("sd80", "man_sd200", 0, vp(0.3, None));
    assert_eq!(s.lookup("sd80", Some("man_sd200"), 0), Some(vp(0.3, None)));
}

#[test]
fn models_and_explicit_defaults_survive_the_file() {
    let mut s = ViewpointStore::default();
    s.set_model("a", "m", 1, vp(0.1, Some(90.0)));
    s.set_vehicle("b", Some("m"), 1, Viewpoint::default());
    let (back, report) = ViewpointStore::parse(&s.to_json());
    assert!(report.problems.is_empty(), "{report:?}");
    assert_eq!(back, s);
    // Without a model entry the default is simply removed.
    let mut t = ViewpointStore::default();
    t.set_vehicle("b", Some("m"), 1, Viewpoint::default());
    assert!(t.vehicles.is_empty());
}

#[test]
fn the_editor_saves_for_the_model_when_asked() {
    let dir = temp_dir("model-scope");
    let path = dir.join("viewpoints.json");
    let mut a = app(path.clone());
    a.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    let e = spawn_vehicle(&mut a, "sd77");
    a.world_mut()
        .entity_mut(e)
        .insert(ViewpointModel("man_sd200".into()));
    a.update();
    edit_and_save(&mut a, |s| {
        s.adjust(opts_bevy::ViewpointRow::Y, 6.0);
        s.scope_model = true;
    });
    a.update();

    // A different vehicle of the same model starts with it.
    let mut b = app(path.clone());
    b.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    let e = spawn_vehicle(&mut b, "sd80");
    b.world_mut()
        .entity_mut(e)
        .insert(ViewpointModel("man_sd200".into()));
    b.update();
    assert_eq!(b.world().resource::<ViewpointState>().active.y, 0.06);
    // A vehicle of another model does not.
    let mut c = app(path);
    c.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    let e = spawn_vehicle(&mut c, "other");
    c.world_mut()
        .entity_mut(e)
        .insert(ViewpointModel("other_model".into()));
    c.update();
    assert!(c.world().resource::<ViewpointState>().active.is_default());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_default_fov_setting_applies_unless_the_viewpoint_has_its_own() {
    let mut st = ViewpointState {
        default_fov_deg: 70.0,
        ..default()
    };
    assert_eq!(st.effective_fov_deg(), 70.0);
    st.user_default_fov_deg = Some(95.0);
    assert_eq!(st.effective_fov_deg(), 95.0);
    st.active.fov_deg = Some(60.0);
    assert_eq!(st.effective_fov_deg(), 60.0);
}
