//! The pause behaviour of the report: F1 pauses, gates and opens the modal, and a paused simulation stays paused.

use super::*;

#[test]
fn f1_pauses_gates_opens_and_cancel_resumes() {
    let mut app = app();
    assert_eq!(state(&app), ReportModalState::Inactive);
    press_f1(&mut app);
    assert_eq!(state(&app), ReportModalState::Capturing);
    assert!(
        app.world().resource::<ModalActive>().0,
        "input is gated while capturing"
    );
    assert!(app.world().resource::<Time<Virtual>>().is_paused());

    deliver_frame(&mut app);
    assert!(app.world().resource::<ModalActive>().0);
    let roots = app
        .world_mut()
        .query::<&opts_bevy::reporting::modal::ReportModalRoot>()
        .iter(app.world())
        .count();
    assert_eq!(roots, 1, "exactly one modal");

    // Escape / Cancel
    let any_modal = app
        .world_mut()
        .query_filtered::<Entity, With<ModalRoot>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .write_message(ModalEvent::CancelRequested(any_modal));
    app.update();
    app.update();
    assert_eq!(state(&app), ReportModalState::Inactive);
    assert!(!app.world().resource::<ModalActive>().0);
    assert!(
        !app.world().resource::<Time<Virtual>>().is_paused(),
        "resumed because it was running"
    );
    let roots = app
        .world_mut()
        .query::<&opts_bevy::reporting::modal::ReportModalRoot>()
        .iter(app.world())
        .count();
    assert_eq!(roots, 0, "modal removed");
}

#[test]
fn an_already_paused_simulation_stays_paused_after_the_modal() {
    let mut app = app();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    press_f1(&mut app);
    deliver_frame(&mut app);
    let m = app
        .world_mut()
        .query_filtered::<Entity, With<ModalRoot>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .write_message(ModalEvent::CancelRequested(m));
    app.update();
    app.update();
    assert_eq!(state(&app), ReportModalState::Inactive);
    assert!(
        app.world().resource::<Time<Virtual>>().is_paused(),
        "must not be unpaused by the modal"
    );
}
#[test]
fn gating_condition_reads_the_modal_flag() {
    let mut world = World::new();
    let mut sys = IntoSystem::into_system(not_modal_active);
    sys.initialize(&mut world);
    assert!(sys.run((), &mut world).unwrap(), "no resource = not gated");
}
