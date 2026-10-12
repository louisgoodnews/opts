//! One notification service for toasts, banners and dialogs
//! (`docs/src/roadmap/phase-9/interaction-and-feel.md` section 5).
//!
//! * [`NotifyExt::notify`] puts a finished event with no decision into a **toast**: stacked (at
//!   most three visible), de-duplicated by message and variables, below modals and above content
//!   (`opts_gui::components::toast_stack`).
//! * [`NotifyExt::notify_error`] **requires the durable place** of the error ([`Durable`]): an
//!   error is never only a toast. A banner is shown by the service; for an inline message or a
//!   dialog the owner has already put it there and says so.

use bevy::prelude::*;
use opts_gui::components::toast::{ToastBuilder, ToastSeverity};
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::Localization;

use super::banner::{show_banner, BannerSpec};
use super::nav::NavigationRefused;

/// Where an error stays after its toast is gone.
#[derive(Debug, Clone)]
pub enum Durable {
    /// A banner on the screen, shown by the service.
    Banner(BannerSpec),
    /// A message next to the field or row that failed (named for the log); the owner shows it.
    Inline(&'static str),
    /// A dialog that asks for a decision; the owner opens it.
    Dialog(&'static str),
}

/// Toast helpers on [`Commands`].
pub trait NotifyExt {
    /// A toast with the message `key` and its variables; identical ones are merged.
    fn notify(
        &mut self,
        severity: ToastSeverity,
        key: &'static str,
        args: Vec<(&'static str, String)>,
    );

    /// An error: a toast **and** its durable place.
    fn notify_error(
        &mut self,
        key: &'static str,
        args: Vec<(&'static str, String)>,
        durable: Durable,
    );
}

impl NotifyExt for Commands<'_, '_> {
    fn notify(
        &mut self,
        severity: ToastSeverity,
        key: &'static str,
        args: Vec<(&'static str, String)>,
    ) {
        self.queue(move |world: &mut World| {
            let formatted: Vec<(String, Arg)> = args
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone().into()))
                .collect();
            let text = world
                .get_resource::<Localization>()
                .map_or_else(|| format!("⟦{key}⟧"), |l| l.t_args(key, &formatted));
            let dedupe = format!(
                "{key}:{}",
                args.iter()
                    .map(|(_, v)| v.as_str())
                    .collect::<Vec<_>>()
                    .join("|")
            );
            let mut commands = world.commands();
            ToastBuilder::new(text)
                .with_severity(severity)
                .with_dedupe_key(dedupe)
                .spawn(&mut commands);
            world.flush();
        });
    }

    fn notify_error(
        &mut self,
        key: &'static str,
        args: Vec<(&'static str, String)>,
        durable: Durable,
    ) {
        self.notify(ToastSeverity::Error, key, args);
        match durable {
            Durable::Banner(spec) => show_banner(self, spec),
            Durable::Inline(place) | Durable::Dialog(place) => {
                tracing::debug!(place, "error shown in its owner's durable place");
            }
        }
    }
}

/// A refused navigation is told to the user (the rail already shows why it is disabled; this
/// covers the control protocol, deep links and shortcuts).
pub fn refusal_toasts(
    mut refused: MessageReader<NavigationRefused>,
    loc: Res<Localization>,
    mut commands: Commands,
) {
    for r in refused.read() {
        let name = super::keys::nav_title(&r.screen).map_or_else(|| r.screen.clone(), |k| loc.t(k));
        commands.notify(
            ToastSeverity::Warning,
            super::keys::NAV_REFUSED,
            vec![("screen", name)],
        );
    }
}

#[cfg(test)]
mod tests {
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
}
