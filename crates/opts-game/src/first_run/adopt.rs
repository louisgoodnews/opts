//! Saves a confirmed installation root and starts the catalogue with it.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;

use crate::catalogue_service::start_catalogue_service;
use crate::omsi_root::set_saved_root;
use crate::settings::GameSettings;
use crate::shell::banner::{dismiss_banner, show_banner, BannerSeverity, BannerSpec};
use crate::shell::keys;

use super::BANNER_SAVE;

/// Saves `root` as the confirmed installation and (re)starts the catalogue with it.
pub fn adopt_root(
    commands: &mut Commands,
    settings: &mut GameSettings,
    root: &std::path::Path,
    persist: bool,
) {
    tracing::info!(path = %root.display(), "OMSI installation confirmed");
    settings.omsi_path = Some(root.to_path_buf());
    set_saved_root(Some(root.to_path_buf()));
    if !persist {
        return;
    }
    if let Err(e) = crate::settings::save(settings) {
        tracing::warn!("could not save the OMSI path: {e}");
        show_banner(
            commands,
            BannerSpec {
                id: BANNER_SAVE,
                severity: BannerSeverity::Warning,
                title_key: keys::FIRSTRUN_SAVE_FAILED_TITLE,
                body_key: keys::FIRSTRUN_SAVE_FAILED_BODY,
                args: L10nArgs::one("error", e.to_string()),
                action: None,
                owner: None,
            },
        );
    } else {
        dismiss_banner(commands, BANNER_SAVE);
    }
    commands.run_system_cached(start_catalogue_service);
}
