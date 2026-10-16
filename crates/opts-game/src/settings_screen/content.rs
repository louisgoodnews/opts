//! The actions of the Content and Advanced pages: choose the OMSI folder, scan now, detect the
//! hardware again.

use bevy::prelude::*;
use opts_gui::components::toast::ToastBuilder;
use opts_gui::l10n::Localization;

use super::input::SettingActionRequested;
use crate::catalogue_service::{spawn_discovery, CatalogueService};
use crate::first_run::OmsiFolderChosen;
use crate::settings_registry::{ChangeSource, SettingsApi};

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
    service: Option<Res<CatalogueService>>,
    mut hardware: Option<ResMut<crate::hardware::HardwareState>>,
) {
    for r in requests.read() {
        match r.key {
            // The same flow as the first run and the Home banner (`first_run`).
            "omsi_path" => crate::first_run::choose_omsi_folder(&mut commands),
            "content.rescan" => match &service {
                Some(s) => {
                    spawn_discovery(s);
                    toast(&mut commands, &loc, "settings-toast-rescan-started");
                }
                None => toast(&mut commands, &loc, "settings-toast-no-omsi"),
            },
            // Detect again and keep the result in the local profile file the user can delete.
            "advanced.hardware_redetect" => {
                if let Some(h) = hardware.as_mut() {
                    h.start_probe(true, true);
                }
                toast(&mut commands, &loc, "settings-toast-hw-detecting");
            }
            "advanced.hardware_delete" => {
                let removed = hardware.as_mut().is_some_and(|h| h.delete_stored());
                toast(
                    &mut commands,
                    &loc,
                    if removed {
                        "settings-toast-hw-deleted"
                    } else {
                        "settings-toast-hw-nothing-stored"
                    },
                );
            }
            _ => {}
        }
    }
}

/// The shell's folder flow validated, saved and adopted a folder: the settings registry (and so
/// the page row) follows. A refused folder is explained by the flow itself (banner or dialog).
pub fn sync_chosen_folder(mut chosen: MessageReader<OmsiFolderChosen>, mut api: SettingsApi) {
    for OmsiFolderChosen(answer) in chosen.read() {
        let Ok(root) = answer else { continue };
        let text = root.display().to_string();
        if let Err(e) = api.change("omsi_path", Some(&text), ChangeSource::Page, true) {
            tracing::warn!(err = %e, "the registry did not take the chosen OMSI folder");
        }
    }
}
