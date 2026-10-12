//! The CI rules of `expandability.md` section 7 on the screens that exist today.
//!
//! Home and the rail are held to every rule. The other screens (new game, settings, HUD, loading) still
//! have hard-coded 11 px texts and small targets; they are recorded in
//! `docs/dev/progress/ui-widgets.md` and join this file when the shell milestone rebuilds them.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState, help::HelpPlugin, home::HomePlugin, settings::GameSettings,
    shell::ShellPlugin,
};
use opts_gui::prelude::*;
use opts_gui::testing::rules;

fn menu_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::light())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(HomePlugin)
        .add_plugins(HelpPlugin);
    app.update(); // start-up: the shell frame exists before any screen is built
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..4 {
        app.update();
    }
    app
}

#[test]
fn shell_meets_the_target_size_rule() {
    let mut app = menu_app();
    let bad = rules::small_targets(app.world_mut());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn shell_disabled_controls_say_why() {
    let mut app = menu_app();
    assert!(rules::disabled_without_reason(app.world_mut()).is_empty());
}

#[test]
fn shell_is_fully_keyboard_operable() {
    let mut app = menu_app();
    let report = rules::keyboard_traversal(&mut app);
    assert!(
        report.missed.is_empty(),
        "never focused: {:?}",
        report.missed
    );
    for id in [
        "shell.nav.home",
        "shell.nav.drive",
        "shell.nav.library",
        "shell.nav.workshop",
        "shell.nav.settings",
        "shell.nav.help",
        "shell.theme-toggle",
        "shell.quit",
        "home.new-drive",
        "home.load-game",
    ] {
        assert!(
            report.reached.iter().any(|r| r == id),
            "{id} not reached: {:?}",
            report.reached
        );
    }
    assert!(
        report.inert.is_empty(),
        "Confirm did nothing on {:?}",
        report.inert
    );
}

#[test]
fn home_starts_on_the_safe_default_and_never_on_quit() {
    let mut app = menu_app();
    let snap = opts_gui::ui_tree::snapshot(app.world_mut());
    assert_eq!(snap.focus.as_deref(), Some("home.new-drive"));
}

#[test]
fn shell_has_no_text_below_12_px() {
    let mut app = menu_app();
    let bad = rules::small_text(app.world_mut());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
