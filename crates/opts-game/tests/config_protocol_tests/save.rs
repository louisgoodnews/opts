//! Saving, replacing and refusing configurations of the plan.


use super::*;


#[test]
pub(super) fn a_saved_configuration_comes_back_into_the_plan_of_another_session() {
    let mut f = fixture("round-trip");
    f.app
        .world_mut()
        .resource_mut::<NewGameSetup>()
        .custom_setvars
        .insert("vis_x".into(), 0);
    let saved = call(
        &mut f.app,
        "config.save",
        json!({ "name": "Line 33", "notes": "late" }),
    )
    .expect("saves");
    assert_eq!(saved["id"], "line-33");
    assert_eq!(saved["replaced"], false);
    assert_eq!(saved["configuration"]["vehicle"], "citybus");
    assert_eq!(saved["configuration"]["vehicle_name"], "Mercedes O530");

    // Another session: the plan is something else.
    *f.app.world_mut().resource_mut::<NewGameSetup>() = NewGameSetup {
        selected_vehicle: "Solaris Urbino".into(),
        ..NewGameSetup::default()
    };
    let listed = call(&mut f.app, "config.list", json!({})).expect("lists");
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["configurations"][0]["usable"], true);
    assert_eq!(listed["configurations"][0]["problems"], json!([]));

    let loaded = call(&mut f.app, "config.load", json!({ "id": "line 33" })).expect("loads");
    assert_eq!(loaded["context"], "wizard");
    let s = setup(&f.app);
    assert_eq!(s.selected_vehicle, "Mercedes O530");
    assert_eq!(s.selected_livery, "Blue");
    assert_eq!(s.custom_setvars.get("vis_x"), Some(&0));
    // The wizard builds its body and the repaint data again.
    assert_eq!(f.app.world().resource::<PlannerState>().built, None);
    assert!(f.app.world().resource::<LiveryState>().repaints.is_none());
}

#[test]
pub(super) fn a_taken_name_needs_replace_and_keeps_the_identity() {
    let mut f = fixture("replace");
    let first = call(&mut f.app, "config.save", json!({ "name": "Day" })).expect("saves");
    call(
        &mut f.app,
        "config.update",
        json!({ "id": "day", "favourite": true }),
    )
    .expect("marks");
    let e = call(&mut f.app, "config.save", json!({ "name": "DAY" })).expect_err("refused");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let again = call(
        &mut f.app,
        "config.save",
        json!({ "name": "DAY", "replace": true }),
    )
    .expect("replaces");
    assert_eq!(again["id"], first["id"]);
    assert_eq!(again["replaced"], true);
    let listed = call(&mut f.app, "config.list", json!({})).expect("lists");
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["configurations"][0]["favourite"], true);
}

#[test]
pub(super) fn nothing_to_save_without_a_vehicle() {
    let mut f = fixture("no-vehicle");
    f.app.world_mut().resource_mut::<NewGameSetup>().no_vehicle = true;
    let e = call(&mut f.app, "config.save", json!({ "name": "x" })).expect_err("refused");
    assert_eq!(e.code, code::NOT_READY);
    let e = call(&mut f.app, "config.save", json!({ "name": "  " })).expect_err("refused");
    assert_eq!(e.code, code::INVALID_PARAMS);
}

#[test]
pub(super) fn a_vehicle_that_is_gone_refuses_the_load_and_changes_nothing() {
    let mut f = fixture("gone");
    call(&mut f.app, "config.save", json!({ "name": "Old" })).expect("saves");
    // The bus is removed from the installation.
    f.app
        .world_mut()
        .resource_mut::<GameCatalogue>()
        .vehicles
        .retain(|v| v.id != "citybus");
    let before = setup(&f.app);
    let listed = call(&mut f.app, "config.list", json!({})).expect("lists");
    assert_eq!(listed["configurations"][0]["usable"], false);
    assert_eq!(
        listed["configurations"][0]["problems"][0]["code"],
        "vehicle_missing"
    );
    let e = call(&mut f.app, "config.load", json!({ "id": "old" })).expect_err("refused");
    assert_eq!(e.code, code::FAILED);
    assert!(e.message.contains("not installed"), "{}", e.message);
    assert_eq!(setup(&f.app), before);
}
