//! The folder dialog: it opens on its own thread, and its answer is checked and saved.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;

use crate::settings::GameSettings;
use crate::shell::banner::{BannerSeverity, BannerSpec, dismiss_banner, show_banner};
use crate::shell::keys;

use super::{BANNER_FOLDER, ChooseError, FirstRun, Step, adopt::adopt_root, check_choice};

/// The answer of the folder dialog (also sent when the choice came from Settings or Home): the
/// installation root, already saved and in use, or the reason it was refused.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct OmsiFolderChosen(pub Result<PathBuf, ChooseError>);

/// The open native dialog, if any.
#[derive(Resource, Default)]
pub struct FolderPicker(pub(super) Mutex<Option<Receiver<Option<PathBuf>>>>);

impl FolderPicker {
    /// Whether a dialog is open (or its answer is not taken yet).
    pub fn is_open(&self) -> bool {
        self.0.lock().is_ok_and(|slot| slot.is_some())
    }
}

/// What shows the folder dialog (the native one; tests substitute a fake that answers at once).
#[derive(Resource, Clone)]
pub struct FolderOpener(pub std::sync::Arc<dyn Fn() -> Option<PathBuf> + Send + Sync>);

impl Default for FolderOpener {
    fn default() -> Self {
        Self(std::sync::Arc::new(|| {
            rfd::FileDialog::new()
                .set_title(select_title())
                .pick_folder()
        }))
    }
}

/// Opens the native folder dialog on its own thread. Anyone may call it (the Settings page for
/// *Content -> OMSI folder*, the Home banner); the result arrives as [`OmsiFolderChosen`].
pub fn choose_omsi_folder(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        let opener = world
            .get_resource::<FolderOpener>()
            .cloned()
            .unwrap_or_default();
        let picker = world.get_resource_or_insert_with(FolderPicker::default);
        let Ok(mut slot) = picker.0.lock() else {
            return;
        };
        if slot.is_some() {
            return; // a dialog is already open
        }
        let (tx, rx) = channel();
        *slot = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send((opener.0)());
        });
    });
}

/// Title of the native dialog (the OS dialog is outside the message tables).
pub(super) fn select_title() -> &'static str {
    "OMSI 2" // l10n-ok: the product name; the dialog's own chrome is the OS's
}

/// Takes the dialog's answer, saves a valid folder and starts the catalogue.
pub fn poll_folder_pick(
    mut commands: Commands,
    picker: Option<Res<FolderPicker>>,
    mut settings: ResMut<GameSettings>,
    mut first_run: ResMut<FirstRun>,
    mut chosen: MessageWriter<OmsiFolderChosen>,
) {
    let Some(picker) = picker else { return };
    let answer = {
        let Ok(mut slot) = picker.0.lock() else {
            return;
        };
        let Some(rx) = slot.as_ref() else { return };
        match rx.try_recv() {
            Ok(answer) => {
                *slot = None;
                answer
            }
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                *slot = None;
                None
            }
        }
    };
    match check_choice(answer) {
        Ok(root) => {
            adopt_root(&mut commands, &mut settings, &root, first_run.persist);
            if matches!(first_run.step, Some(Step::Choose)) {
                first_run.step = None;
            }
            first_run.error = None;
            dismiss_banner(&mut commands, BANNER_FOLDER);
            chosen.write(OmsiFolderChosen(Ok(root)));
        }
        Err(ChooseError::Cancelled) => {
            chosen.write(OmsiFolderChosen(Err(ChooseError::Cancelled)));
        }
        Err(error @ ChooseError::NotInstallation(_)) => {
            let ChooseError::NotInstallation(path) = &error else {
                return;
            };
            // Durable: the dialog shows it inline; elsewhere a banner keeps it until resolved.
            if first_run.step.is_some() {
                first_run.step = Some(Step::Choose);
                first_run.error = Some(error.clone());
            } else {
                show_banner(
                    &mut commands,
                    BannerSpec {
                        id: BANNER_FOLDER,
                        severity: BannerSeverity::Error,
                        title_key: keys::FIRSTRUN_ERROR_TITLE,
                        body_key: keys::FIRSTRUN_WRONG_FOLDER,
                        args: L10nArgs::one("path", path.display().to_string()),
                        action: None,
                        owner: None,
                    },
                );
            }
            chosen.write(OmsiFolderChosen(Err(error)));
        }
    }
}
