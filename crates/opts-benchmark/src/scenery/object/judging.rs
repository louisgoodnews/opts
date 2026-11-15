//! Combining load evidence and view metrics into the object's verdict.

use super::*;

/// Combines load-stage evidence and metrics into the verdict.
pub(super) fn judge(
    h: &Harness,
    record: &mut Record,
    pre: &Preflight,
    exp: &Expected,
    diagnostics: super::super::Diagnostics,
    text: Option<TextCheck>,
) {
    let mut load_cats = pre.categories.clone();
    load_cats.extend(exp.categories.iter().cloned());
    load_cats.extend(diag::load_categories(
        &diagnostics.events,
        &diagnostics.stats,
    ));
    if exp.keyed.total > 0 {
        record.class.push("keyed".into());
    }
    if record.class.iter().any(|c| c == "editor-only") && record.class.iter().any(|c| c == "tree") {
        record.tags.info.push("I-EDITOR-TREE".into());
    }
    if exp.lod_groups > 1 {
        record.tags.info.push("F-LOD-OVERDRAW".into());
    }
    let has_expected = !exp.tris.is_empty();
    let reason = (!has_expected).then(|| no_expected_reason(pre, exp));
    let decision = decide(
        &h.config.thresholds,
        &Evidence {
            classes: &record.class,
            load_categories: &load_cats,
            has_expected,
            no_expected_reason: reason,
            views: &record.views,
            text: text.as_ref().map(|t| (&t.changed_share, &t.px_text)),
        },
    );
    record.verdict = decision.verdict;
    record.categories = decision.categories;
    record.unknown_reason = decision.unknown_reason;
    record.text_check = text;
    record.diagnostics = diagnostics;
}

pub(super) fn no_expected_reason(pre: &Preflight, exp: &Expected) -> String {
    if exp.keyed.unresolved > 0 {
        "keyed mesh the generator does not cover".into()
    } else if pre.classes.iter().any(|c| c == "vehicle-model") {
        "[model] objects are not predicted".into()
    } else {
        "no expected silhouette".into()
    }
}
