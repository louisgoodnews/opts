//! Failure categories and the verdict (`docs/src/architecture/scenery-benchmark.md`, Static check).

use std::collections::BTreeMap;

use super::{Verdict, ViewMetrics};

mod rules;
mod thresholds;
pub use thresholds::Thresholds;

use rules::{geometry_rules, text_rule};

/// Text-check evidence: mean colour difference and text pixels per view.
pub type TextEvidence<'a> = (&'a BTreeMap<String, f32>, &'a BTreeMap<String, usize>);

/// Everything the verdict depends on.
#[derive(Debug, Clone)]
pub struct Evidence<'a> {
    /// Context classes.
    pub classes: &'a [String],
    /// Load-stage categories (preflight and diagnostics).
    pub load_categories: &'a [String],
    /// Whether an expected silhouette exists.
    pub has_expected: bool,
    /// Why there is none, if there is none.
    pub no_expected_reason: Option<String>,
    /// Per-view metrics.
    pub views: &'a BTreeMap<String, ViewMetrics>,
    /// Text check: mean difference per view (`None`: not applicable).
    pub text: Option<TextEvidence<'a>>,
}

/// Verdict, categories and the reason for `unknown`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Decision {
    /// Verdict.
    pub verdict: Verdict,
    /// Failure categories.
    pub categories: Vec<String>,
    /// Why the verdict is `unknown`.
    pub unknown_reason: Option<String>,
}

pub(super) fn push(v: &mut Vec<String>, s: &str) {
    if !v.iter().any(|x| x == s) {
        v.push(s.to_string());
    }
}

/// Applies the rules to the evidence.
#[must_use]
pub fn decide(t: &Thresholds, ev: &Evidence<'_>) -> Decision {
    let mut cats: Vec<String> = Vec::new();
    for c in ev.load_categories {
        push(&mut cats, c);
    }
    let has_class = |c: &str| ev.classes.iter().any(|x| x == c);
    let rendered = ev.views.values().map(|v| v.px_rendered).max().unwrap_or(0);
    let mut unknown = None;
    // A `[tree]` billboard is drawn although the file says `[onlyeditor]` (Q-SB-13).
    if has_class("editor-only") && !has_class("tree") {
        if rendered >= t.empty_px {
            push(&mut cats, "F-EDITOR-RENDERED");
        }
    } else if has_class("help-arrow") {
        // Route arrows are drawn only when OMSI's helper arrows are switched on: no rendering
        // is required, but a missing texture still counts.
        if ev
            .views
            .values()
            .any(|v| v.sentinel_share > t.sentinel_share)
        {
            push(&mut cats, "F-TEX-MISSING");
        }
    } else if !ev.has_expected {
        let expect_nothing = has_class("no-geometry") || has_class("carpark-marker");
        if expect_nothing && rendered >= t.empty_px {
            push(&mut cats, "F-GEOM-OUTSIDE");
        } else if !expect_nothing {
            unknown = ev
                .no_expected_reason
                .clone()
                .or(Some("no expected silhouette".into()));
        }
    } else {
        unknown = geometry_rules(t, ev, &mut cats);
    }
    // Skipped or missing meshes explain the geometry shortfalls that follow from them.
    if cats
        .iter()
        .any(|c| c == "F-MESH-PROTECTED" || c == "F-MESH-MISSING")
    {
        cats.retain(|c| {
            !matches!(
                c.as_str(),
                "F-RENDER-EMPTY" | "F-GEOM-PARTIAL" | "F-XFORM-SCALE" | "F-XFORM-OFFSET"
            )
        });
    }
    // An invisible object is explained by its missing texture; a text check needs the texture.
    if cats.iter().any(|c| c == "F-TEX-MISSING") {
        cats.retain(|c| c != "F-RENDER-EMPTY" && c != "F-TEXT-NOT-SHOWN");
    } else if !has_class("help-arrow") {
        text_rule(t, ev, &mut cats);
    }
    let verdict = if !cats.is_empty() {
        Verdict::Fail
    } else if unknown.is_some() {
        Verdict::Unknown
    } else {
        Verdict::Pass
    };
    Decision {
        verdict,
        categories: cats,
        unknown_reason: if verdict == Verdict::Unknown {
            unknown
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests;
