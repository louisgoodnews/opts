//! Read-only information lines of the settings pages (plain words, no setting behind them):
//! the current theme, the catalogue, the cache, the file locations and the menu keys.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;
use opts_gui::prelude::*;

use super::rows::{label, Ctx};
use crate::settings::GameSettings;
use crate::settings_registry::Category;

/// A message line with variables.
fn line(
    c: &mut Commands,
    parent: Entity,
    key: &'static str,
    args: &[(&str, String)],
    style: TypeScale,
) {
    let e = label(c, parent, key, style, ColorRole::OnSurfaceVariant);
    let mut a = L10nArgs::default();
    for (k, v) in args {
        a.0.push(((*k).to_string(), v.clone().into()));
    }
    c.entity(e).insert(a);
}

fn path_text(p: Option<std::path::PathBuf>) -> String {
    p.map_or_else(String::new, |p| p.display().to_string())
}

/// A Unix timestamp as local date and time.
fn local_time<T: TryInto<i64>>(t: T) -> String {
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

/// Bytes shown as megabytes with one decimal.
fn megabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_048_576.0)
}

/// Files and bytes below `dir`.
fn dir_usage(dir: &std::path::Path) -> (usize, u64) {
    let mut n = (0, 0);
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                if m.is_file() {
                    n.0 += 1;
                    n.1 += m.len();
                }
            }
        }
    }
    n
}

/// The detected hardware (M-0081): what OPTS knows about this machine, and where it is stored.
fn hardware_info(c: &mut Commands, pane: Entity, ctx: &Ctx) {
    label(
        c,
        pane,
        "settings-info-hw-title",
        TypeScale::TitleMedium,
        ColorRole::OnSurface,
    );
    label(
        c,
        pane,
        "settings-info-hw-privacy",
        TypeScale::BodyMedium,
        ColorRole::OnSurfaceVariant,
    );
    let Some(profile) = ctx.hardware else {
        label(
            c,
            pane,
            "settings-info-hw-detecting",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        );
        return;
    };
    for (key, vars) in crate::hardware::view::lines(profile, ctx.loc) {
        let args: Vec<(&str, String)> = vars;
        line(c, pane, key, &args, TypeScale::BodyMedium);
    }
    match &ctx.hardware_file {
        Some(p) => line(
            c,
            pane,
            "settings-info-hw-stored",
            &[("path", p.display().to_string())],
            TypeScale::BodyMedium,
        ),
        None => {
            label(
                c,
                pane,
                "settings-info-hw-not-stored",
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            );
        }
    }
}

/// Information that belongs to `cat`, below the rows.
pub fn spawn_info(c: &mut Commands, pane: Entity, ctx: &Ctx, cat: Category, theme_kind: &str) {
    match cat {
        Category::Interface => {
            line(
                c,
                pane,
                "settings-info-theme-now",
                &[("theme", ctx.loc.t(theme_kind))],
                TypeScale::BodyMedium,
            );
            c.queue(|world: &mut World| {
                // The last spawned text of the page is the line above: mark it so it follows
                // the theme while the page is open.
                let mut q = world.query::<(Entity, &opts_gui::l10n::L10nKey)>();
                let found: Vec<Entity> = q
                    .iter(world)
                    .filter(|(_, k)| k.0 == "settings-info-theme-now")
                    .map(|(e, _)| e)
                    .collect();
                for e in found {
                    world.entity_mut(e).insert(super::page::ThemeNowText);
                }
            });
        }
        Category::Content => {
            let summary = crate::catalogue_service::catalogue_db_path()
                .filter(|p| p.exists())
                .and_then(|p| opts_catalogue::Catalogue::open(&p).ok())
                .map(|cat| {
                    (
                        cat.totals().unwrap_or((0, 0)),
                        cat.last_scan().ok().flatten(),
                    )
                });
            let ((files, entities), last) = summary.unwrap_or(((0, 0), None));
            line(
                c,
                pane,
                "settings-info-catalogue",
                &[
                    ("files", files.to_string()),
                    ("entities", entities.to_string()),
                ],
                TypeScale::BodyMedium,
            );
            line(
                c,
                pane,
                if last.is_some() {
                    "settings-info-last-scan"
                } else {
                    "settings-info-never-scanned"
                },
                &[("when", last.map(local_time).unwrap_or_default())],
                TypeScale::BodyMedium,
            );
            let (n, bytes) =
                opts_bevy::descramble_cache::directory().map_or((0, 0), |d| dir_usage(&d));
            line(
                c,
                pane,
                "settings-info-cache",
                &[("files", n.to_string()), ("size", megabytes(bytes))],
                TypeScale::BodyMedium,
            );
        }
        Category::Advanced => {
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
                "settings-info-cache-folder",
                &[("path", path_text(opts_bevy::descramble_cache::directory()))],
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
        Category::Controls => {
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
