//! Thresholds, failure categories and the verdict
//! (`docs/src/architecture/scenery-benchmark.md`, Static check).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Verdict, ViewMetrics};

/// Decision thresholds (recorded in `run.json`; calibrated values in the spec, Q-SB-02).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    /// Fewer rendered pixels than this counts as an empty render.
    pub empty_px: usize,
    /// Expected pixels needed before an empty render is a failure.
    pub min_expected_px: usize,
    /// `F-GEOM-OUTSIDE` below this precision.
    pub precision: f32,
    /// `F-GEOM-OUTSIDE` above this bounding-box overshoot.
    pub bbox_overshoot: f32,
    /// `F-XFORM-ROTATION`: identity IoU below this …
    pub rotation_identity_iou: f32,
    /// … and an alternative orientation at or above this.
    pub rotation_best_iou: f32,
    /// `F-XFORM-OFFSET` above this centroid offset (both views).
    pub centroid_offset: f32,
    /// `F-XFORM-SCALE` outside `[1 / scale, scale]` (both views).
    pub scale: f32,
    /// `F-GEOM-PARTIAL` below this recall.
    pub recall: f32,
    /// Recall threshold for mostly alpha-tested objects; between it and `recall` is `unknown`.
    pub recall_alpha: f32,
    /// An object is mostly alpha-tested above this alpha share.
    pub alpha_share: f32,
    /// `F-TEX-MISSING` above this sentinel share.
    pub sentinel_share: f32,
    /// `F-TEXT-NOT-SHOWN` below this mean colour difference.
    pub text_diff: f32,
    /// The text check needs at least this many text pixels in a view.
    pub text_min_px: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            empty_px: 20,
            min_expected_px: 200,
            precision: 0.90,
            bbox_overshoot: 0.25,
            rotation_identity_iou: 0.5,
            rotation_best_iou: 0.8,
            centroid_offset: 0.10,
            scale: 1.25,
            recall: 0.6,
            recall_alpha: 0.2,
            alpha_share: 0.5,
            sentinel_share: 0.02,
            text_diff: 0.02,
            text_min_px: 20,
        }
    }
}

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

fn push(v: &mut Vec<String>, s: &str) {
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
    if has_class("editor-only") {
        if rendered >= t.empty_px {
            push(&mut cats, "F-EDITOR-RENDERED");
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
    text_rule(t, ev, &mut cats);
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

/// The silhouette rules; returns an `unknown` reason for borderline alpha-tested recall.
fn geometry_rules(t: &Thresholds, ev: &Evidence<'_>, cats: &mut Vec<String>) -> Option<String> {
    let judged: Vec<&ViewMetrics> = ev
        .views
        .values()
        .filter(|v| v.px_expected >= t.min_expected_px)
        .collect();
    if judged.is_empty() {
        return Some("expected silhouette too small to judge".into());
    }
    if judged.iter().all(|v| v.px_rendered < t.empty_px) {
        push(cats, "F-RENDER-EMPTY");
        return None;
    }
    let mut unknown = None;
    for v in &judged {
        if v.precision.is_some_and(|p| p < t.precision)
            || v.bbox_overshoot.is_some_and(|o| o > t.bbox_overshoot)
        {
            push(cats, "F-GEOM-OUTSIDE");
        }
        if v.sentinel_share > t.sentinel_share {
            push(cats, "F-TEX-MISSING");
        }
        if let Some(r) = v.recall {
            if v.alpha_share > t.alpha_share {
                if r < t.recall_alpha {
                    push(cats, "F-GEOM-PARTIAL");
                } else if r < t.recall {
                    unknown = Some("alpha-tested object with borderline recall".to_string());
                }
            } else if r < t.recall {
                push(cats, "F-GEOM-PARTIAL");
            }
        }
    }
    let all = |f: &dyn Fn(&ViewMetrics) -> bool| judged.iter().all(|v| f(v));
    if all(&|v| v.centroid_offset.is_some_and(|c| c > t.centroid_offset)) {
        push(cats, "F-XFORM-OFFSET");
    }
    let off_scale = |r: f32| r > t.scale || r < 1.0 / t.scale;
    if all(&|v| {
        v.size_ratio
            .is_some_and(|s| off_scale(s[0]) || off_scale(s[1]))
    }) {
        push(cats, "F-XFORM-SCALE");
    }
    if let Some(v2) = ev.views.get("v2") {
        let rotated = v2.iou.is_some_and(|i| i < t.rotation_identity_iou)
            && v2
                .best_rotation
                .as_ref()
                .is_some_and(|(_, iou)| *iou >= t.rotation_best_iou);
        if rotated {
            push(cats, "F-XFORM-ROTATION");
        }
    }
    unknown
}

fn text_rule(t: &Thresholds, ev: &Evidence<'_>, cats: &mut Vec<String>) {
    let Some((diff, px)) = ev.text else { return };
    let judged: Vec<f32> = diff
        .iter()
        .filter(|(view, _)| px.get(*view).is_some_and(|n| *n >= t.text_min_px))
        .map(|(_, d)| *d)
        .collect();
    if !judged.is_empty() && judged.iter().all(|d| *d < t.text_diff) {
        push(cats, "F-TEXT-NOT-SHOWN");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(rendered: usize, precision: f32, recall: f32) -> ViewMetrics {
        ViewMetrics {
            px_rendered: rendered,
            px_expected: 1000,
            px_expected_base: 1000,
            precision: Some(precision),
            recall: Some(recall),
            iou: Some(0.9),
            bbox_overshoot: Some(0.0),
            centroid_offset: Some(0.01),
            size_ratio: Some([1.0, 1.0]),
            ..ViewMetrics::default()
        }
    }

    fn decide_views(v1: ViewMetrics, v2: ViewMetrics) -> Decision {
        let classes = vec!["standalone".to_string()];
        let views: BTreeMap<String, ViewMetrics> = [("v1".into(), v1), ("v2".into(), v2)].into();
        let ev = Evidence {
            classes: &classes,
            load_categories: &[],
            has_expected: true,
            no_expected_reason: None,
            views: &views,
            text: None,
        };
        decide(&Thresholds::default(), &ev)
    }

    #[test]
    fn good_render_passes() {
        let d = decide_views(view(950, 0.99, 0.97), view(900, 0.98, 0.95));
        assert_eq!(d.verdict, Verdict::Pass);
    }

    #[test]
    fn empty_and_partial_renders_fail() {
        let d = decide_views(view(0, 0.0, 0.0), view(3, 0.0, 0.0));
        assert_eq!(d.categories, vec!["F-RENDER-EMPTY"]);
        let d = decide_views(view(400, 0.99, 0.4), view(900, 0.99, 0.95));
        assert_eq!(d.categories, vec!["F-GEOM-PARTIAL"]);
    }

    #[test]
    fn offset_needs_both_views() {
        let mut a = view(900, 0.95, 0.95);
        a.centroid_offset = Some(0.3);
        let d = decide_views(a.clone(), view(900, 0.95, 0.95));
        assert!(!d.categories.contains(&"F-XFORM-OFFSET".to_string()));
        let d = decide_views(a.clone(), a);
        assert!(d.categories.contains(&"F-XFORM-OFFSET".to_string()));
    }

    #[test]
    fn scale_needs_both_views_and_outside_is_caught() {
        let mut a = view(900, 0.95, 0.95);
        a.size_ratio = Some([2.0, 2.0]);
        let d = decide_views(a.clone(), a);
        assert!(d.categories.contains(&"F-XFORM-SCALE".to_string()));
        let d = decide_views(view(900, 0.5, 0.95), view(900, 0.95, 0.95));
        assert!(d.categories.contains(&"F-GEOM-OUTSIDE".to_string()));
    }

    #[test]
    fn rotation_uses_the_alternative_orientation() {
        let mut v2 = view(900, 0.95, 0.95);
        v2.iou = Some(0.2);
        v2.best_rotation = Some(("heading-mirrored".into(), 0.95));
        let d = decide_views(view(900, 0.95, 0.95), v2);
        assert!(d.categories.contains(&"F-XFORM-ROTATION".to_string()));
    }

    #[test]
    fn sentinel_pixels_mean_a_missing_texture() {
        let mut v = view(900, 0.95, 0.95);
        v.sentinel_share = 0.3;
        let d = decide_views(v.clone(), v);
        assert!(d.categories.contains(&"F-TEX-MISSING".to_string()));
    }

    #[test]
    fn unchanged_text_is_reported() {
        let classes = vec!["standalone".to_string()];
        let views: BTreeMap<String, ViewMetrics> = [("v1".into(), view(900, 0.99, 0.97))].into();
        let diff: BTreeMap<String, f32> = [("v1".into(), 0.0)].into();
        let px: BTreeMap<String, usize> = [("v1".into(), 500)].into();
        let ev = Evidence {
            classes: &classes,
            load_categories: &[],
            has_expected: true,
            no_expected_reason: None,
            views: &views,
            text: Some((&diff, &px)),
        };
        let d = decide(&Thresholds::default(), &ev);
        assert!(d.categories.contains(&"F-TEXT-NOT-SHOWN".to_string()));
    }
}
