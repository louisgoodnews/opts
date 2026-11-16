//! The report modal UI and its button/Escape handling.

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use opts_gui::interaction::{events::ButtonClicked, focus::FocusEvent};
use opts_gui::l10n::{L10nKey, Localization};
use opts_gui::prelude::*;

use super::format::{format_report, now_stamp, ReportMeta};
use super::keys;
use super::save::{save_report, SavedReport};
use super::state::{ReportContext, ReportModalData, ReportModalState};

mod actions;
mod poll;
mod spawn;

pub use actions::{handle_modal_actions, ReportModalActionParams};
pub use poll::poll_save;
pub use spawn::spawn_modal;

/// Marker on the modal's backdrop (despawned recursively on close).
#[derive(Component)]
pub struct ReportModalRoot;
/// Marker on the Report button.
#[derive(Component)]
pub struct ReportButton;
/// Marker on the Cancel button.
#[derive(Component)]
pub struct CancelButton;

/// Where reports are written (resolved and created at start-up).
#[derive(Resource, Debug, Default, Clone)]
pub struct ReportsDir(pub Option<std::path::PathBuf>);

/// A save running on the task pool.
#[derive(Resource, Default)]
pub struct SaveTask(Option<Task<std::io::Result<SavedReport>>>);

/// The inline error line of the dialog.
#[derive(Component)]
pub struct ReportErrorText;

/// The message `key` in the user's language, with `args`; the English fallback without tables.
pub(super) fn tr(loc: Option<&Localization>, key: (&str, &str), args: &[(&str, String)]) -> String {
    let formatted: Vec<(String, opts_gui::l10n::fluent::Arg)> = args
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone().into()))
        .collect();
    loc.and_then(|l| l.format(key.0, &formatted))
        .unwrap_or_else(|| {
            args.iter().fold(key.1.to_owned(), |text, (k, v)| {
                text.replace(&format!("{{ ${k} }}"), v)
            })
        })
}

/// z-index above the modal so failure toasts stay visible.
const TOAST_Z: i32 = 2000;

/// Despawns the modal (also when a capture failed and none exists).
pub fn despawn_modal(mut commands: Commands, roots: Query<Entity, With<ReportModalRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}

/// Keeps `ReportModalData::text` in sync with the text area.
pub fn sync_text(mut events: MessageReader<TextAreaChanged>, mut data: ResMut<ReportModalData>) {
    for ev in events.read() {
        data.text.clone_from(&ev.text);
    }
}

/// A failure is never only a toast: it stays in the dialog as well.
pub(super) fn fail(commands: &mut Commands, data: &mut ReportModalData, text: String) {
    data.error = Some(text.clone());
    let toast = ToastBuilder::new(text).with_duration(8.0).spawn(commands);
    commands.entity(toast).insert(GlobalZIndex(TOAST_Z));
}

/// Shows the current error in the dialog.
pub fn sync_error(data: Res<ReportModalData>, mut lines: Query<&mut Text, With<ReportErrorText>>) {
    if !data.is_changed() {
        return;
    }
    for mut line in &mut lines {
        let text = data.error.clone().unwrap_or_default();
        if line.0 != text {
            line.0 = text;
        }
    }
}
