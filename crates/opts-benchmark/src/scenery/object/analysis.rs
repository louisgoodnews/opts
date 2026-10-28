//! Silhouette, rotation and dynamic-text analysis for one placed scenery object.

use super::*;

/// Mattes, masks and metrics of both views.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// Per-view mattes (`v1`, `v2`).
    pub mattes: Vec<Matte>,
    /// Per-view expected masks.
    pub expected: Vec<ExpectedMasks>,
    /// Per-view metrics.
    pub metrics: BTreeMap<String, super::super::ViewMetrics>,
    /// Text close-ups (`t1_a`, `t1_b`, `t2_a`, `t2_b`), kept as images on request.
    pub text_shots: Vec<(String, Matte)>,
}

pub(super) fn analyse(
    px: &[ViewPixels; 2],
    exp: &Expected,
    place: &Placement,
    views: &[View; 2],
) -> Analysis {
    let mut out = Analysis {
        mattes: Vec::new(),
        expected: Vec::new(),
        metrics: BTreeMap::new(),
        text_shots: Vec::new(),
    };
    for (i, (p, view)) in px.iter().zip(views).enumerate() {
        let matte = metrics::matte(&p.black, &p.white);
        let masks = metrics::expected_masks(&exp.tris, place, view);
        let mut m = metrics::view_metrics(&matte, &masks);
        if i == 1 && m.iou.is_some_and(|v| v < 0.5) {
            m.best_rotation = best_rotation(&matte, exp, place, view, m.iou.unwrap_or(0.0));
        }
        out.metrics.insert(format!("v{}", i + 1), m);
        out.mattes.push(matte);
        out.expected.push(masks);
    }
    out
}

/// The alternative orientation whose expected silhouette matches the render best, if it beats
/// the identity.
fn best_rotation(
    r: &Matte,
    exp: &Expected,
    place: &Placement,
    view: &View,
    identity: f32,
) -> Option<(String, f32)> {
    let mut alternatives: Vec<(String, Placement)> = [90.0, 180.0, 270.0]
        .iter()
        .map(|d| {
            let mut p = *place;
            p.heading_deg += d;
            (format!("heading+{d}"), p)
        })
        .collect();
    let mut mirrored = *place;
    mirrored.heading_deg = -place.heading_deg;
    alternatives.push(("heading-mirrored".into(), mirrored));
    let mut mirror_x = *place;
    mirror_x.mirror_x = true;
    alternatives.push(("mirror-x".into(), mirror_x));
    alternatives
        .into_iter()
        .filter_map(|(name, p)| {
            let e = metrics::expected_masks(&exp.tris, &p, view).all;
            let union = r.mask.or_count(&e);
            (union > 0).then(|| (name, r.mask.and_count(&e) as f32 / union as f32))
        })
        .filter(|(_, iou)| *iou > identity)
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// Looks at the text-textured triangles from both sides with string set A and again with set B
/// and measures how much of the text region changed. Placement A must still be loaded.
pub(super) fn text_check(
    h: &mut Harness,
    entry: &ObjectEntry,
    pre: &Preflight,
    exp: &Expected,
    place: &Placement,
    tile_place: mapgen::Placement,
) -> Option<(TextCheck, Vec<(String, Matte)>)> {
    if !pre.has_text() {
        return None;
    }
    let text_tris: Vec<[Vec3; 3]> = exp
        .tris
        .iter()
        .filter(|t| t.flags.text)
        .map(|t| t.v.map(|v| place.to_map(v)))
        .collect();
    let views = text_views(&text_tris)?;
    let a = h.capture(&views).ok()?;
    let strings_b = pre.strings.strings('B');
    h.place(Some((&entry.sco, &strings_b, tile_place))).ok()?;
    let deadline = Instant::now() + h.config.object_timeout;
    if !h.settle(deadline).unwrap_or(false) {
        return None;
    }
    let b = h.capture(&views).ok()?;
    let delta = h.config.thresholds.text_delta;
    let mut check = TextCheck::default();
    let mut shots = Vec::new();
    for (i, view) in views.iter().enumerate() {
        let (ma, mb) = (
            metrics::matte(&a[i].black, &a[i].white),
            metrics::matte(&b[i].black, &b[i].white),
        );
        let region = metrics::expected_masks(&exp.tris, place, view).text;
        let name = format!("t{}", i + 1);
        check.px_text.insert(name.clone(), region.count());
        if let Some(d) = metrics::mean_colour_diff(&ma, &mb, &region) {
            check.mean_diff.insert(name.clone(), d);
        }
        if let Some(c) = metrics::changed_share(&ma, &mb, &region, delta) {
            check.changed_share.insert(name.clone(), c);
        }
        shots.push((format!("{name}_a"), ma));
        shots.push((format!("{name}_b"), mb));
    }
    Some((check, shots))
}
