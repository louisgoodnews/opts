//! The Advanced page's file locations and the hardware block below them.

use bevy::prelude::*;
use opts_gui::prelude::*;

use super::super::rows::{label, Ctx};
use super::{hardware_info, line, path_text};
use crate::settings::GameSettings;

/// The hardware profile and the places of the files.
pub(super) fn advanced_info(c: &mut Commands, pane: Entity, ctx: &Ctx) {
    hardware_info(c, pane, ctx);
    label(
        c,
        pane,
        "settings-info-locations",
        TypeScale::TitleMedium,
        ColorRole::OnSurface,
    );
    line(
        c,
        pane,
        "settings-info-settings-file",
        &[("path", path_text(GameSettings::database_path()))],
        TypeScale::BodyMedium,
    );
    line(
        c,
        pane,
        "settings-info-reference-file",
        &[("path", path_text(GameSettings::toml_reference_path()))],
        TypeScale::BodyMedium,
    );
    line(
        c,
        pane,
        "settings-info-catalogue-file",
        &[(
            "path",
            path_text(crate::catalogue_service::catalogue_db_path()),
        )],
        TypeScale::BodyMedium,
    );
    line(
        c,
        pane,
        "settings-info-themes-folder",
        &[("path", path_text(crate::theme_files::themes_dir()))],
        TypeScale::BodyMedium,
    );
    line(
        c,
        pane,
        "settings-info-last-map",
        &[(
            "path",
            ctx.settings
                .recent_map_path
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        )],
        TypeScale::BodyMedium,
    );
}
