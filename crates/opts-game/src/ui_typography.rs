//! The user's text preferences (M-0109): the UI font family and the base text size.
//!
//! * `ui_font`: a font family put in front of the bundled UI stack (empty: Noto Sans). The choices
//!   are the families the font database holds beyond the UI family: the other bundled faces,
//!   every font installed on the computer (Bevy's `system_font_discovery` feature, which links
//!   `libfontconfig` on Linux) and every font file in the `fonts/` folder of the configuration
//!   directory (`*.ttf`, `*.otf`), which is read once at start-up and registered like the bundled
//!   ones. A family that is not in the database is refused on the spot: the previous one stays and
//!   an error toast says so.
//! * `ui_font_size`: the body text size, 12 to 24 px (default 14); every style of the type scale
//!   follows it and none goes below 12 px (`opts_gui::fonts::UiTypography`).

mod choices;
mod fonts;

use choices::{list_choices, refuse_unknown_font};
use fonts::load_user_fonts;
pub use fonts::read_font_files;

use std::path::PathBuf;

use bevy::prelude::*;
use directories::ProjectDirs;
use opts_gui::fonts::{UiTypography, BASE_TEXT_PX, BASE_TEXT_RANGE};

use crate::settings::GameSettings;

/// Key of the font family setting.
pub const SETTING_FONT: &str = "ui_font";
/// Key of the base text size setting.
pub const SETTING_SIZE: &str = "ui_font_size";

/// The folder user fonts are read from (`<config dir>/fonts`).
#[must_use]
pub fn fonts_dir() -> Option<PathBuf> {
    ProjectDirs::from("org", "opts", "opts").map(|dirs| dirs.config_dir().join("fonts"))
}

/// The families the user can choose besides the default.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct FontChoices(pub Vec<String>);

/// The typography the settings describe: the family as stored, the size clamped to 12 to 24.
#[must_use]
pub fn wanted(settings: &GameSettings) -> UiTypography {
    let size = settings
        .raw(SETTING_SIZE)
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map_or(BASE_TEXT_PX, |p| {
            p.clamp(*BASE_TEXT_RANGE.start(), *BASE_TEXT_RANGE.end()) as f32
        });
    UiTypography {
        family: settings
            .raw(SETTING_FONT)
            .unwrap_or_default()
            .trim()
            .to_owned(),
        base_px: size,
    }
}

/// Registers the font folder, the sync of the settings and the check of a new family.
pub struct UiTypographyPlugin;

impl Plugin for UiTypographyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FontChoices>()
            .add_systems(Startup, load_user_fonts)
            .add_systems(
                PreUpdate,
                sync_typography.before(opts_gui::theme::runtime::resolve_theme),
            )
            .add_systems(Update, (refuse_unknown_font, list_choices));
    }
}

/// Copies the settings into the [`UiTypography`] resource when they change.
fn sync_typography(settings: Res<GameSettings>, mut typography: ResMut<UiTypography>) {
    if !settings.is_changed() {
        return;
    }
    let wanted = wanted(&settings);
    if *typography != wanted {
        *typography = wanted;
    }
}

#[cfg(test)]
mod tests;
