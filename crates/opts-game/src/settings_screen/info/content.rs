//! The Content page's background-fact lines: catalogue numbers, totals and the last scan.

use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

use super::super::rows::label;
use super::local_time;

/// Variables of the two catalogue lines: the breakdown by kind and the files-versus-entries line.
pub(super) type Args = Vec<(&'static str, String)>;

pub(super) fn catalogue_args(c: &opts_catalogue::KindCounts) -> (Args, Args) {
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

/// The content numbers; each shows "Calculating..." until the background facts answer.
pub(super) fn content_info(c: &mut Commands, pane: Entity) {
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
