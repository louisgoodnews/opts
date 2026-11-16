//! The details toast: its *Details* dialog copies the text and offers the report.

use super::*;

#[test]
fn details_toast_dialog_copies_and_reports() {
    let mut app = app();
    app.insert_non_send(opts_gui::components::text_area::TextClipboard::in_memory());
    app.world_mut().write_message(ShowDetailsToast {
        severity: ToastSeverity::Warning,
        message: "Two files failed".into(),
        details: "a.png\nb.png".into(),
    });
    app.update();
    app.update();
    let action = app
        .world_mut()
        .query::<(Entity, &ToastActionButton)>()
        .iter(app.world())
        .map(|(e, _)| e)
        .next()
        .expect("the toast has a Details action");
    assert!(
        app.world_mut()
            .query::<&M3Toast>()
            .iter(app.world())
            .count()
            >= 1
    );
    app.world_mut().write_message(Activated(action));
    app.update();
    app.update();
    let dialog_button = |app: &mut App, choice: &str| {
        app.world_mut()
            .query::<(Entity, &crate::shell::dialog::DialogButton)>()
            .iter(app.world())
            .find(|(_, b)| b.choice == choice)
            .map(|(e, _)| e)
    };
    let copy = dialog_button(&mut app, "copy").expect("Copy button");
    assert!(dialog_button(&mut app, "report").is_some());
    app.world_mut().write_message(Activated(copy));
    app.update();
    app.update();
    assert!(
        dialog_button(&mut app, "copy").is_some(),
        "Copy keeps the dialog open"
    );
    let copied = app
        .world_mut()
        .non_send_mut::<opts_gui::components::text_area::TextClipboard>()
        .get();
    assert_eq!(copied, "Two files failed\n\na.png\nb.png");
    let report = dialog_button(&mut app, "report").expect("Report button");
    app.world_mut().write_message(Activated(report));
    app.update();
    app.update();
    assert!(dialog_button(&mut app, "report").is_none(), "dialog closed");
    let state = *app
        .world()
        .resource::<State<opts_bevy::reporting::ReportModalState>>()
        .get();
    let pending = !matches!(
        app.world()
            .resource::<NextState<opts_bevy::reporting::ReportModalState>>(),
        NextState::Unchanged
    );
    assert!(
        pending || state != opts_bevy::reporting::ReportModalState::Inactive,
        "the report dialog was requested"
    );
}
