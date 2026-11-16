//! Unit tests of the notification service: merged toasts, and errors that keep a durable place.

use super::*;
use crate::shell::banner::BannerSeverity;
use crate::shell::{keys, Navigate, ShellPlugin};
use opts_gui::components::toast::M3Toast;
use opts_gui::l10n::L10nArgs;
use opts_gui::prelude::*;

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
        .init_state::<crate::app_state::AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin);
    app.update();
    app
}

fn toasts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&M3Toast>()
        .iter(app.world())
        .map(|t| t.message.clone())
        .collect()
}

#[test]
fn identical_notifications_become_one_toast_with_a_counter() {
    let mut app = app();
    for _ in 0..3 {
        let mut commands = app.world_mut().commands();
        commands.notify(
            ToastSeverity::Info,
            keys::NAV_REFUSED,
            vec![("screen", "Library".into())],
        );
        app.update();
    }
    app.update();
    assert_eq!(toasts(&mut app).len(), 1);
}

#[test]
fn an_error_is_never_only_a_toast() {
    let mut app = app();
    {
        let mut commands = app.world_mut().commands();
        commands.notify_error(
            keys::NAV_REFUSED,
            vec![("screen", "x".into())],
            Durable::Banner(BannerSpec {
                id: "test.error",
                severity: BannerSeverity::Error,
                title_key: keys::NAV_REFUSED,
                body_key: keys::NAV_REFUSED,
                args: L10nArgs::one("screen", "x"),
                action: None,
                owner: None,
            }),
        );
    }
    app.update();
    app.update();
    assert_eq!(toasts(&mut app).len(), 1);
    let banners = app
        .world_mut()
        .query::<&crate::shell::banner::Banner>()
        .iter(app.world())
        .count();
    assert_eq!(banners, 1, "the durable place exists besides the toast");
}

#[test]
fn a_refused_destination_is_announced_in_the_users_language() {
    let mut app = app();
    app.world_mut().write_message(Navigate::to("library"));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        toasts(&mut app),
        vec!["Library is not available.".to_owned()]
    );
}
