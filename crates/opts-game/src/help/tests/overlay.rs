//! The F1 overlay in a drive, and the translated report dialog keys.

use super::*;

#[test]
fn f1_in_a_drive_opens_the_overlay_over_a_paused_scene_and_closing_returns_to_the_pause_menu() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<crate::in_game::types::SimulationLoadState>()
        .is_complete = true;
    press(&mut app, KeyCode::F1, Key::F1);
    assert!(ids(&mut app).contains(&"help-overlay.close".to_owned()));
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    assert_eq!(state(&app), AppState::InGame);
    let close =
        opts_gui::ui_tree::find_entity(app.world_mut(), "help-overlay.close").expect("close");
    app.world_mut()
        .write_message(opts_gui::interaction::activation::Activated(close));
    for _ in 0..4 {
        app.update();
    }
    assert!(
        ids(&mut app).contains(&"pause.resume".to_owned()),
        "the pause menu is back"
    );
}

#[test]
fn every_report_dialog_key_is_translated() {
    let mut loc = opts_gui::l10n::Localization::new("en");
    for (locale, source) in [
        ("en", include_str!("../../shell/l10n/shell-en.ftl")),
        ("de", include_str!("../../shell/l10n/shell-de.ftl")),
    ] {
        loc.extend(locale, source);
    }
    for (key, fallback) in opts_bevy::reporting::keys::ALL {
        assert!(loc.has("en", key) && loc.has("de", key), "{key}");
        let english = loc.t_args(
            key,
            &[("path".into(), "P".into()), ("error".into(), "E".into())],
        );
        let normalised = fallback
            .replace("{ $path }", "P")
            .replace("{ $error }", "E");
        assert_eq!(
            english, normalised,
            "the fallback of `{key}` matches the table"
        );
    }
}
