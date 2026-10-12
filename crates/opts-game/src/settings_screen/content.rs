//! The actions of the Content and Advanced pages: choose the OMSI folder, scan now, clear the
//! texture cache.

use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use bevy::prelude::*;
use opts_gui::components::toast::ToastBuilder;
use opts_gui::l10n::Localization;

use super::input::SettingActionRequested;
use crate::catalogue_service::{spawn_discovery, CatalogueService};
use crate::settings_registry::{ChangeSource, SettingsApi};

/// Lets the user pick a folder (a native dialog; a fake in tests).
pub trait FolderChooser: Send + Sync {
    /// Blocks until the user picked a folder or cancelled.
    fn choose(&self) -> Option<PathBuf>;
}

/// The dialog of the operating system.
pub struct NativeFolderChooser;

impl FolderChooser for NativeFolderChooser {
    fn choose(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }
}

/// The folder chooser in use.
#[derive(Resource, Clone)]
pub struct FolderChooserRes(pub Arc<dyn FolderChooser>);

impl Default for FolderChooserRes {
    fn default() -> Self {
        Self(Arc::new(NativeFolderChooser))
    }
}

/// A pick in flight on a background thread.
#[derive(Resource, Default)]
pub struct PendingPick(Mutex<Option<mpsc::Receiver<Option<PathBuf>>>>);

fn toast(commands: &mut Commands, loc: &Localization, key: &str) {
    ToastBuilder::new(loc.t(key))
        .with_duration(4.0)
        .spawn(commands);
}

/// Starts the actions asked for by the page.
pub fn run_actions(
    mut commands: Commands,
    mut requests: MessageReader<SettingActionRequested>,
    loc: Res<Localization>,
    chooser: Res<FolderChooserRes>,
    pending: Res<PendingPick>,
    service: Option<Res<CatalogueService>>,
) {
    for r in requests.read() {
        match r.key {
            "omsi_path" => {
                let (tx, rx) = mpsc::channel();
                let chooser = chooser.0.clone();
                if std::thread::Builder::new()
                    .name("folder-chooser".into())
                    .spawn(move || {
                        let _ = tx.send(chooser.choose());
                    })
                    .is_ok()
                {
                    if let Ok(mut p) = pending.0.lock() {
                        *p = Some(rx);
                    }
                }
            }
            "content.rescan" => match &service {
                Some(s) => {
                    spawn_discovery(s);
                    toast(&mut commands, &loc, "settings-toast-rescan-started");
                }
                None => toast(&mut commands, &loc, "settings-toast-no-omsi"),
            },
            "content.clear_cache" => {
                let removed = opts_bevy::descramble_cache::prune(0);
                tracing::info!(removed, "texture cache cleared from the settings page");
                toast(&mut commands, &loc, "settings-toast-cache-cleared");
            }
            _ => {}
        }
    }
}

/// Takes the picked OMSI folder (checked like the first-run prompt) and saves it.
pub fn take_picked_folder(
    mut commands: Commands,
    loc: Res<Localization>,
    pending: Res<PendingPick>,
    mut api: SettingsApi,
) {
    let answer = pending
        .0
        .lock()
        .ok()
        .and_then(|p| p.as_ref().map(mpsc::Receiver::try_recv))
        .and_then(Result::ok);
    let Some(answer) = answer else { return };
    if let Ok(mut p) = pending.0.lock() {
        *p = None;
    }
    let Some(picked) = answer else { return };
    match crate::omsi_root::root_from_selection(&picked) {
        Some(root) => {
            let text = root.display().to_string();
            if api
                .change("omsi_path", Some(&text), ChangeSource::Page, true)
                .is_ok()
            {
                crate::omsi_root::set_saved_root(Some(root));
                toast(&mut commands, &loc, "settings-toast-omsi-restart");
            }
        }
        None => toast(&mut commands, &loc, "settings-toast-omsi-invalid"),
    }
}
