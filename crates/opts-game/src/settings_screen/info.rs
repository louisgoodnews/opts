//! Read-only information lines of the settings pages (plain words, no setting behind them):
//! the current theme, the catalogue, the cache, the file locations and the menu keys.

use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, L10nKey};
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
    let _ = match &ctx.hardware_file {
        Some(p) => line(
            c,
            pane,
            "settings-info-hw-stored",
            &[("path", p.display().to_string())],
            TypeScale::BodyMedium,
        ),
        None => label(
            c,
            pane,
            "settings-info-hw-not-stored",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ),
    };
}

/// Variables of the two catalogue lines: the breakdown by kind and the files-versus-entries line.
type Args = Vec<(&'static str, String)>;

fn catalogue_args(c: &opts_catalogue::KindCounts) -> (Args, Args) {
    (
        vec![
            ("maps", c.maps.to_string()),
            ("vehicles", c.vehicles.to_string()),
            ("scenery", c.scenery_objects.to_string()),
            ("splines", c.splines.to_string()),
            ("other", c.other.to_string()),
        ],
        vec![
            ("entities", c.entities.to_string()),
            ("files", c.files.to_string()),
        ],
    )
}

/// The lines of the Content page that show background facts.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentLine {
    /// The breakdown by kind.
    Catalogue,
    /// Files versus entries.
    Total,
    /// The last scan (or "never").
    LastScan,
}

/// Message key and variables of a content line once its facts are known.
pub(super) fn content_line_text(
    which: ContentLine,
    f: &crate::content_facts::ContentFacts,
) -> (&'static str, Args) {
    let (kinds, total) = catalogue_args(&f.heavy.kinds);
    match which {
        ContentLine::Catalogue => ("settings-info-catalogue", kinds),
        ContentLine::Total => ("settings-info-catalogue-total", total),
        ContentLine::LastScan => match f.light.last_scan {
            Some(t) => ("settings-info-last-scan", vec![("when", local_time(t))]),
            None => ("settings-info-never-scanned", vec![("when", String::new())]),
        },
    }
}

/// Fills the content lines from the facts store as soon as an answer is there (the heavy
/// lines wait for the heavy facts, the scan line for the light ones) and again after each new
/// answer. Runs every frame but only touches entities when something arrived.
pub fn fill_content_lines(
    store: Res<crate::content_facts::ContentFactsStore>,
    mut lines: Query<(Ref<ContentLine>, &mut L10nKey, &mut L10nArgs)>,
) {
    let (Some(light), heavy) = (store.light(), store.heavy()) else {
        return;
    };
    for (which, mut key, mut args) in &mut lines {
        if !store.is_changed() && !which.is_added() {
            continue;
        }
        let which = *which;
        let ready = match which {
            ContentLine::LastScan => true,
            _ => heavy.is_some(),
        };
        if !ready {
            continue;
        }
        let facts = crate::content_facts::ContentFacts {
            light,
            heavy: heavy.unwrap_or_default(),
        };
        let (k, a) = content_line_text(which, &facts);
        key.0 = k;
        args.0 = a
            .into_iter()
            .map(|(n, v)| (n.to_string(), v.into()))
            .collect();
    }
}

/// Information that belongs to `cat`, below the rows.
pub fn spawn_info(c: &mut Commands, pane: Entity, ctx: &Ctx, cat: Category, theme_kind: &str) {
    match cat {
        Category::Interface => {
            let now = line(
                c,
                pane,
                "settings-info-theme-now",
                &[("theme", ctx.loc.t(theme_kind))],
                TypeScale::BodyMedium,
            );
            // The theme file list comes from the background facts.
            c.entity(now)
                .insert(crate::content_facts::WantsContentFacts);
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
            // The numbers come from the background facts (`content_facts`): the page never
            // waits for them. Each line shows "Calculating..." until its answer arrives.
            for (which, key) in [
                (ContentLine::Catalogue, "settings-info-catalogue-pending"),
                (ContentLine::Total, "settings-info-catalogue-total-pending"),
                (ContentLine::LastScan, "settings-info-last-scan-pending"),
            ] {
                let e = label(
                    c,
                    pane,
                    key,
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                );
                c.entity(e).insert((
                    which,
                    L10nArgs::default(),
                    crate::content_facts::WantsContentFacts,
                ));
            }
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

#[cfg(test)]
mod tests;
