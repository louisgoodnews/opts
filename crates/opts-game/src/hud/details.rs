//! Toasts with a *Details* dialog (interaction section 5, audit F22 and F18).
//!
//! A toast is one short line. When more is worth knowing (a list of files, an error text), the
//! owner sends [`ShowDetailsToast`]: the toast carries a *Details* action that opens a dialog with
//! the whole text, a **Copy** button (system clipboard; stays open) and a **Report** button
//! (opens the report dialog with a screenshot and the log, like Help does). *Close* is the
//! first button, so it has the focus and Esc does the same.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_gui::components::text_area::TextClipboard;
use opts_gui::components::toast::{ToastActionButton, ToastBuilder, ToastSeverity};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, Localization};

use super::keys;
use crate::shell::dialog::{
    spawn_confirm_dialog, DialogButtonSpec, DialogButtonStyle, DialogChoice, DialogSpec,
};
use crate::shell::notify::NotifyExt;

/// The dialog's tag.
pub const TAG: &str = "hud.details";

/// Asks for a toast with a *Details* dialog.
#[derive(Message, Debug, Clone)]
pub struct ShowDetailsToast {
    /// Severity of the toast.
    pub severity: ToastSeverity,
    /// The short line of the toast (already in the user's language).
    pub message: String,
    /// The whole text of the dialog.
    pub details: String,
}

/// The details of the toasts that are showing, and of the open dialog.
#[derive(Resource, Debug, Default)]
pub struct DetailsStore {
    toasts: HashMap<Entity, String>,
    /// The text of the dialog that is open.
    pub open: Option<String>,
}

/// Spawns the toasts.
pub fn spawn_details_toasts(
    mut requests: MessageReader<ShowDetailsToast>,
    loc: Res<Localization>,
    mut store: ResMut<DetailsStore>,
    mut commands: Commands,
) {
    for r in requests.read() {
        let toast = ToastBuilder::new(r.message.clone())
            .with_severity(r.severity)
            .with_action(loc.t(keys::DETAILS_ACTION))
            .spawn(&mut commands);
        store
            .toasts
            .insert(toast, format!("{}\n\n{}", r.message, r.details));
    }
}

/// The *Details* action of a toast opens the dialog.
pub fn open_details(
    mut activated: MessageReader<Activated>,
    actions: Query<&ToastActionButton>,
    mut store: ResMut<DetailsStore>,
    mut commands: Commands,
) {
    for Activated(entity) in activated.read() {
        let Ok(action) = actions.get(*entity) else {
            continue;
        };
        let Some(text) = store.toasts.remove(&action.0) else {
            continue;
        };
        store.open = Some(text.clone());
        spawn_confirm_dialog(
            &mut commands,
            DialogSpec {
                tag: TAG,
                title_key: keys::DETAILS_TITLE,
                body_key: keys::DETAILS_BODY,
                body_args: L10nArgs::one("details", text),
                buttons: vec![
                    DialogButtonSpec {
                        choice: "close",
                        label_key: keys::DETAILS_CLOSE,
                        style: DialogButtonStyle::Quiet,
                    },
                    DialogButtonSpec {
                        choice: "copy",
                        label_key: keys::DETAILS_COPY,
                        style: DialogButtonStyle::Primary,
                    },
                    DialogButtonSpec {
                        choice: "report",
                        label_key: keys::DETAILS_REPORT,
                        style: DialogButtonStyle::Primary,
                    },
                ],
                keep_open: vec!["copy"],
                error: None,
            },
        );
    }
}

/// Forgets the toasts that went away.
pub fn forget_dismissed(mut store: ResMut<DetailsStore>, toasts: Query<Entity>) {
    if !store.toasts.is_empty() {
        store.toasts.retain(|e, _| toasts.contains(*e));
    }
}

/// Copy and Report.
#[allow(clippy::too_many_arguments)]
pub fn details_choices(
    mut choices: MessageReader<DialogChoice>,
    mut store: ResMut<DetailsStore>,
    clipboard: Option<NonSendMut<TextClipboard>>,
    time: Res<Time<Virtual>>,
    data: Option<ResMut<opts_bevy::reporting::ReportModalData>>,
    state: Option<Res<State<opts_bevy::reporting::ReportModalState>>>,
    next: Option<ResMut<NextState<opts_bevy::reporting::ReportModalState>>>,
    mut commands: Commands,
) {
    let mut clipboard = clipboard;
    let (mut data, mut next) = (data, next);
    for c in choices.read().filter(|c| c.tag == TAG) {
        match c.choice {
            "copy" => {
                if let (Some(text), Some(clip)) = (store.open.as_deref(), clipboard.as_mut()) {
                    clip.set(text);
                    commands.notify(ToastSeverity::Info, keys::DETAILS_COPIED, Vec::new());
                }
            }
            "report" => {
                if let (Some(data), Some(state), Some(next)) =
                    (data.as_mut(), state.as_ref(), next.as_mut())
                {
                    opts_bevy::reporting::request_report(&time, data, state, next);
                }
                store.open = None;
            }
            _ => store.open = None,
        }
    }
}
