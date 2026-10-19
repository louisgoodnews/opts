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

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::text::FontCx;
use directories::ProjectDirs;
use opts_gui::fonts::{
    UiTypography, BASE_TEXT_PX, BASE_TEXT_RANGE, FALLBACK_FAMILY, MONO_FAMILY, UI_FAMILY,
};

use crate::settings::GameSettings;
use crate::settings_registry::{ChangeSource, SettingChanged};
use crate::shell::notify::{Durable, NotifyExt};

/// Key of the font family setting.
pub const SETTING_FONT: &str = "ui_font";
/// Key of the base text size setting.
pub const SETTING_SIZE: &str = "ui_font_size";
/// Most font files read from the folder.
const MAX_FILES: usize = 32;
/// Largest font file accepted (bytes).
const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;

/// The folder user fonts are read from (`<config dir>/fonts`).
#[must_use]
pub fn fonts_dir() -> Option<PathBuf> {
    ProjectDirs::from("org", "opts", "opts").map(|dirs| dirs.config_dir().join("fonts"))
}

/// The font files of `dir` (`*.ttf`, `*.otf`, sorted by name): file name and bytes. Files over
/// 25 MB and everything beyond the first 32 files are left out.
#[must_use]
pub fn read_font_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.eq_ignore_ascii_case("ttf") || x.eq_ignore_ascii_case("otf"))
        })
        .collect();
    names.sort();
    let mut out = Vec::new();
    for path in names.into_iter().take(MAX_FILES) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match std::fs::metadata(&path) {
            Ok(m) if m.len() <= MAX_FILE_BYTES => match std::fs::read(&path) {
                Ok(bytes) => out.push((name, bytes)),
                Err(e) => tracing::warn!(%name, "font file cannot be read: {e}"),
            },
            Ok(_) => tracing::warn!(%name, "font file is over 25 MB; skipped"),
            Err(e) => tracing::warn!(%name, "font file cannot be read: {e}"),
        }
    }
    out
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

/// Reads the font folder once and registers every file as a font asset.
fn load_user_fonts(fonts: Option<ResMut<Assets<Font>>>) {
    let (Some(mut fonts), Some(dir)) = (fonts, fonts_dir()) else {
        return;
    };
    let files = read_font_files(&dir);
    if !files.is_empty() {
        tracing::info!(count = files.len(), dir = %dir.display(), "user fonts");
    }
    for (_, bytes) in files {
        fonts.add(Font::from_bytes(bytes));
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

/// Whether the font database holds the family.
fn has_family(cx: &mut FontCx, name: &str) -> bool {
    cx.collection.family_id(name).is_some()
}

/// A family the database does not hold is given back: the previous one stays, an error toast says so.
fn refuse_unknown_font(
    mut changes: MessageReader<SettingChanged>,
    font_cx: Option<ResMut<FontCx>>,
    mut commands: Commands,
) {
    let Some(mut cx) = font_cx else {
        for _ in changes.read() {}
        return;
    };
    for c in changes.read() {
        if c.key != SETTING_FONT || c.new.trim().is_empty() || c.source == ChangeSource::Revert {
            continue;
        }
        if has_family(&mut cx, c.new.trim()) {
            continue;
        }
        tracing::warn!(family = %c.new, "the font family is not installed; keeping the previous one");
        let old = c.old.clone();
        // Through the world: this system reads the change messages, the revert writes one.
        commands.queue(move |world: &mut World| {
            if let Err(e) = crate::settings_registry::apply_change(
                world,
                SETTING_FONT,
                Some(&old),
                ChangeSource::Revert,
            ) {
                tracing::warn!("could not restore the font setting: {e}");
            }
        });
        commands.notify_error(
            "settings-font-unavailable",
            vec![("font", c.new.clone())],
            Durable::Inline("settings row ui_font"),
        );
    }
}

/// Keeps [`FontChoices`] in step with the font database when fonts are added.
fn list_choices(
    fonts: Option<Res<Assets<Font>>>,
    font_cx: Option<ResMut<FontCx>>,
    mut choices: ResMut<FontChoices>,
    mut seen: Local<usize>,
) {
    let (Some(fonts), Some(mut cx)) = (fonts, font_cx) else {
        return;
    };
    if *seen == fonts.len() && *seen != 0 {
        return;
    }
    *seen = fonts.len();
    let mut names: Vec<String> = cx
        .collection
        .family_names()
        .filter(|n| !n.starts_with("asset_id:") && *n != "Material Icons" && *n != UI_FAMILY)
        .map(str::to_owned)
        .collect();
    // The other bundled faces first, then what the folder added.
    names.sort_by_key(|n| {
        (
            ![FALLBACK_FAMILY, MONO_FAMILY].contains(&n.as_str()),
            n.to_lowercase(),
        )
    });
    names.dedup();
    if choices.0 != names {
        choices.0 = names;
    }
}

#[cfg(test)]
mod tests;
