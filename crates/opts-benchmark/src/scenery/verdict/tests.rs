//! Unit tests for `super` (`verdict.rs`).

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
