//! Read-only information lines of the settings pages (plain words, no setting behind them):
//! the current theme, the catalogue, the cache, the file locations and the menu keys.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;
use opts_gui::prelude::*;

use super::rows::{label, Ctx};
use crate::settings_registry::Category;

mod content;
mod hardware;
mod interface;
mod locations;

pub use content::{fill_content_lines, ContentLine};
use content::content_info;
#[cfg(test)]
use content::{catalogue_args, Args};
use hardware::hardware_info;
use interface::interface_info;
use locations::advanced_info;

/// A message line with variables.
fn line(
    c: &mut Commands,
    parent: Entity,
    key: &'static str,
    args: &[(&str, String)],
    style: TypeScale,
) -> Entity {
    let e = label(c, parent, key, style, ColorRole::OnSurfaceVariant);
    let mut a = L10nArgs::default();
    for (k, v) in args {
        a.0.push(((*k).to_string(), v.clone().into()));
    }
    c.entity(e).insert(a);
    e
}

fn path_text(p: Option<std::path::PathBuf>) -> String {
    p.map_or_else(String::new, |p| p.display().to_string())
}

/// A Unix timestamp as local date and time.
pub(super) fn local_time<T: TryInto<i64>>(t: T) -> String {
    t.try_into()
        .ok()
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

/// The key lines.
fn controls_info(c: &mut Commands, pane: Entity) {
    label(
        c,
        pane,
        "settings-keys-title",
        TypeScale::TitleMedium,
        ColorRole::OnSurface,
    );
    for k in KEY_LINES {
        label(
            c,
            pane,
            k,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        );
    }
}

/// Information that belongs to `cat`, below the rows.
pub fn spawn_info(c: &mut Commands, pane: Entity, ctx: &Ctx, cat: Category, theme_kind: &str) {
    match cat {
        Category::Interface => interface_info(c, pane, ctx, theme_kind),
        Category::Content => content_info(c, pane),
        Category::Advanced => advanced_info(c, pane, ctx),
        Category::Controls => controls_info(c, pane),
        _ => {}
    }
}

/// The keys and buttons of the menus (the action layer's tables), one message each.
pub const KEY_LINES: [&str; 9] = [
    "settings-key-navigate",
    "settings-key-regions",
    "settings-key-confirm",
    "settings-key-back",
    "settings-key-search",
    "settings-key-tabs",
    "settings-key-scroll",
    "settings-key-menu",
    "settings-key-developer",
];

#[cfg(test)]
mod tests;
