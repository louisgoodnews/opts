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
    for (group, rows) in super::controls::GROUPS {
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

#[test]
fn the_controls_table_shows_the_current_keys() {
    let mut app = app();
    let mut keys = opts_bevy::ActionKeys::default();
    keys.set("help", opts_bevy::KeyChord::parse("Ctrl+H").unwrap());
    app.insert_resource(keys);
    enter_help(&mut app);
    for _ in 0..3 {
        app.update();
    }
    let shown: Vec<String> = app
        .world_mut()
        .query_filtered::<&Text, With<super::controls::ActionChip>>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(shown.iter().any(|t| t == "Ctrl+H"), "{shown:?}");
    assert!(!shown.iter().any(|t| t == "F1"), "{shown:?}");
    assert!(
        shown.iter().any(|t| t == "F6"),
        "the others keep theirs: {shown:?}"
    );
}

fn all_texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect()
}

#[test]
fn the_vehicle_part_lists_the_current_vehicle_keys_and_the_cockpit_controls() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<crate::settings::GameSettings>()
        .set_raw("controls.vehicle.lights", "Ctrl+G");
    let owner = app.world_mut().spawn(opts_bevy::PlayerVehicle).id();
    for name in ["cp_kluczyk_mov", "horn", "horn"] {
        app.world_mut().spawn(opts_bevy::MouseTrigger {
            owner,
            trigger: name.into(),
        });
    }
    enter_help(&mut app);
    for _ in 0..4 {
        app.update();
    }
    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "Ctrl+G"),
        "the rebound lights key: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "M"),
        "the default engine start key"
    );
    assert!(
        !texts.iter().any(|t| t == "L"),
        "the old lights key is gone"
    );
    assert_eq!(
        texts.iter().filter(|t| *t == "horn").count(),
        1,
        "a name once"
    );
    assert!(texts.iter().any(|t| t == "cp_kluczyk_mov"), "{texts:?}");
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
