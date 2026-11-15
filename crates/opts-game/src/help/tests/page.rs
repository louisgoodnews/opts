//! The Help page: its tabs, the deep link, F1 and the rebound key.

use super::*;

#[test]
fn the_page_offers_four_tabs_and_starts_on_the_generated_controls() {
    let mut app = app();
    enter_help(&mut app);
    let ids = ids(&mut app);
    for tab in ["controls", "guides", "report", "about"] {
        assert!(ids.contains(&format!("help.tab.{tab}")), "{tab}: {ids:?}");
    }
    let shown = keys_shown(&mut app);
    assert!(shown.contains(&keys::HELP_KEY_THROTTLE));
    // The table is generated from the bindings registry: every row is on the page.
    for (group, rows) in crate::help::controls::GROUPS {
        assert!(shown.contains(group), "{group}");
        for row in *rows {
            assert!(shown.contains(&row.action), "{}", row.shown);
        }
    }
}

#[test]
fn tabs_switch_the_pane_and_about_shows_the_version_and_licences() {
    let mut app = app();
    enter_help(&mut app);
    for (tab, key) in [
        ("about", keys::HELP_ABOUT_LICENCE),
        ("guides", keys::HELP_GUIDES_BODY),
        ("report", keys::HELP_REPORT_BODY),
    ] {
        let button =
            opts_gui::ui_tree::find_entity(app.world_mut(), &format!("help.tab.{tab}")).expect(tab);
        app.world_mut()
            .write_message(opts_gui::interaction::activation::Activated(button));
        for _ in 0..3 {
            app.update();
        }
        assert!(keys_shown(&mut app).contains(&key), "{tab}");
    }
    assert!(ids(&mut app).contains(&"help.report".to_owned()));
}

#[test]
fn a_deep_link_opens_the_named_tab() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<crate::shell::nav::PendingDeepLink>()
        .0 = Some("about".to_owned());
    enter_help(&mut app);
    assert_eq!(app.world().resource::<HelpState>().tab, HelpTab::About);
}

#[test]
fn f1_opens_the_help_page_from_the_shell() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..3 {
        app.update();
    }
    press(&mut app, KeyCode::F1, Key::F1);
    assert_eq!(state(&app), AppState::Help);
}

#[test]
fn a_rebound_help_key_opens_the_help_and_the_old_key_does_not() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..3 {
        app.update();
    }
    let mut keys = opts_bevy::ActionKeys::default();
    keys.set("help", opts_bevy::KeyChord::parse("H").unwrap());
    app.insert_resource(keys);
    press(&mut app, KeyCode::F1, Key::F1);
    assert_eq!(state(&app), AppState::MainMenu, "F1 is free now");
    press(&mut app, KeyCode::KeyH, Key::Character("h".into()));
    assert_eq!(state(&app), AppState::Help);
}
