//! Help: tabs, F1, the overlay, the report keys.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

use super::*;
use crate::pause::PausePlugin;
use crate::shell::ShellPlugin;

fn app() -> App {
    crate::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(crate::settings::GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .init_resource::<crate::in_game::VehicleMenu>()
        .init_resource::<crate::in_game::types::SimulationLoadState>()
        .init_resource::<crate::new_game_screen::types::NewGameSetup>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(PausePlugin)
        .add_plugins(HelpPlugin);
    app.update();
    app
}

fn press(app: &mut App, code: KeyCode, logical: Key) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: logical.clone(),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        for _ in 0..3 {
            app.update();
        }
    }
}

fn state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn ids(app: &mut App) -> Vec<String> {
    opts_gui::ui_tree::snapshot(app.world_mut())
        .flatten()
        .iter()
        .map(|n| n.id.clone())
        .collect()
}

fn enter_help(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Help);
    for _ in 0..4 {
        app.update();
    }
}

fn keys_shown(app: &mut App) -> Vec<&'static str> {
    app.world_mut()
        .query::<&L10nKey>()
        .iter(app.world())
        .map(|k| k.0)
        .collect()
}

#[test]
fn the_page_offers_four_tabs_and_starts_on_controls_with_the_not_yet_line() {
    let mut app = app();
    enter_help(&mut app);
    let ids = ids(&mut app);
    for tab in ["controls", "guides", "report", "about"] {
        assert!(ids.contains(&format!("help.tab.{tab}")), "{tab}: {ids:?}");
    }
    let shown = keys_shown(&mut app);
    assert!(shown.contains(&keys::HELP_KEY_THROTTLE));
    assert!(
        shown.contains(&opts_gui::l10n::keys::NOT_AVAILABLE),
        "generated list is marked"
    );
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
        app.world_mut().write_message(Activated(button));
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
    app.world_mut().resource_mut::<PendingDeepLink>().0 = Some("about".to_owned());
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
    app.world_mut().write_message(Activated(close));
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
        ("en", include_str!("../shell/l10n/shell-en.ftl")),
        ("de", include_str!("../shell/l10n/shell-de.ftl")),
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
