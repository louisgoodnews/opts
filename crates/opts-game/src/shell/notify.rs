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
mod tests;
