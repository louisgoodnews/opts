//! The OMSI folder button and the folder that the shell flow adopts.

use super::*;

#[test]
fn the_omsi_folder_button_starts_the_shells_folder_flow() {
    let (mut app, _) = content_app("omsi-flow");
    assert!(!app
        .world()
        .resource::<crate::first_run::FolderPicker>()
        .is_open());
    let b = by_id(&mut app, "settings/content/omsi_path/choose").expect("choose button");
    click(&mut app, b);
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world()
            .resource::<crate::first_run::FolderPicker>()
            .is_open(),
        "first_run::choose_omsi_folder ran"
    );
}

#[test]
fn a_folder_the_shell_flow_adopted_reaches_the_registry() {
    let root = std::env::temp_dir().join(format!("opts-settings-omsi-{}", std::process::id()));
    let (mut app, db) = content_app("omsi-ok");
    app.world_mut()
        .write_message(crate::first_run::OmsiFolderChosen(Ok(root.clone())));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(stored(&db, "omsi_path").as_deref(), root.to_str());
}

#[test]
fn a_refused_folder_changes_nothing() {
    let (mut app, db) = content_app("omsi-bad");
    app.world_mut()
        .write_message(crate::first_run::OmsiFolderChosen(Err(
            crate::first_run::ChooseError::Cancelled,
        )));
    for _ in 0..3 {
        app.update();
    }
    assert!(stored(&db, "omsi_path").is_none_or(|v| v.is_empty()));
}
