//! The silhouette and text rules of the verdict: which failure categories the evidence triggers.

use super::super::ViewMetrics;
use super::{push, Evidence, Thresholds};

/// The silhouette rules; returns an `unknown` reason when alpha coverage cannot be verified.
pub(super) fn geometry_rules(
    t: &Thresholds,
    ev: &Evidence<'_>,
    cats: &mut Vec<String>,
) -> Option<String> {
    // Billboards and text-textured boards are mostly gaps: their coverage is not predictable.
    let sparse = ev.classes.iter().any(|c| c == "tree") || ev.text.is_some();
    let judged: Vec<&ViewMetrics> = ev
        .views
        .values()
        .filter(|v| v.px_expected >= t.min_expected_px)
        .collect();
    if judged.is_empty() {
        return Some("expected silhouette too small to judge".into());
    }
    let alpha = |v: &ViewMetrics| v.alpha_share > t.alpha_share;
    if judged.iter().all(|v| v.px_rendered < t.empty_px) {
        if judged.iter().all(|v| alpha(v)) {
            return Some("alpha-tested object rendered nothing (coverage not verifiable)".into());
        }
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
            if alpha(v) {
                // Other alpha objects (signs, fences) that cover well under the mesh area
                // deserve a human look.
                let floor = if sparse { t.recall_alpha } else { t.recall };
                if r < floor {
                    unknown = Some("alpha-tested object below the expected coverage".to_string());
                }
            } else if r < t.recall {
                push(cats, "F-GEOM-PARTIAL");
            }
        }
    }
    // The visible part of an alpha texture (glyphs, leaves) sits anywhere in its quad.
    let offset =
        |v: &ViewMetrics| !alpha(v) && v.centroid_offset.is_some_and(|c| c > t.centroid_offset);
    if judged.iter().all(|v| offset(v)) {
        push(cats, "F-XFORM-OFFSET");
    }
    // The visible part of an alpha texture may be much smaller than its quad: only growth counts.
    let off_scale = |v: &ViewMetrics, r: f32| r > t.scale || (!alpha(v) && r < 1.0 / t.scale);
    if judged.iter().all(|v| {
        v.size_ratio
            .is_some_and(|s| off_scale(v, s[0]) || off_scale(v, s[1]))
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

pub(super) fn text_rule(t: &Thresholds, ev: &Evidence<'_>, cats: &mut Vec<String>) {
    let Some((diff, px)) = ev.text else { return };
    let judged: Vec<f32> = diff
        .iter()
        .filter(|(view, _)| px.get(*view).is_some_and(|n| *n >= t.text_min_px))
        .map(|(_, d)| *d)
        .collect();
    // Shown when the text changed in at least one close-up (front or back).
    if !judged.is_empty() && judged.iter().all(|d| *d < t.text_changed_share) {
        push(cats, "F-TEXT-NOT-SHOWN");
    }
}
