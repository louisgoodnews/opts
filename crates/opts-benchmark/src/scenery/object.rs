//! The per-object loop: preflight, place, settle, diagnose, render, analyse, record
//! (`docs/src/architecture/scenery-benchmark.md`, Per-object loop).

use std::collections::BTreeMap;
use std::time::Instant;

use bevy::math::Vec3;
use opts_game::in_game::omsi_world::WorldStats;

use super::capture::ViewPixels;
use super::expected::{self, Expected, Placement};
use super::framing::{oblique_view, text_views, top_view, Bounds, View};
use super::mapgen::{self, OBJECT_HEADING_DEG, OBJECT_POS};
use super::metrics::{self, ExpectedMasks, Matte};
use super::preflight::{self, Preflight};
use super::verdict::{decide, Evidence};
use super::worker::Harness;
use super::{
    diag, images, script_check, script_run, Audio, ObjectEntry, Record, Tags, TextCheck, Timing,
};

/// Milliseconds since `t`.
fn ms(t: Instant) -> u64 {
    t.elapsed().as_millis() as u64
}

/// The `WorldStats` counters that change while an object loads, as deltas.
fn stats_delta(before: &WorldStats, after: &WorldStats) -> BTreeMap<String, i64> {
    let pairs = [
        (
            "objects_loaded",
            before.objects_loaded,
            after.objects_loaded,
        ),
        (
            "objects_failed",
            before.objects_failed,
            after.objects_failed,
        ),
        (
            "objects_protected",
            before.objects_protected,
            after.objects_protected,
        ),
        (
            "objects_no_geometry",
            before.objects_no_geometry,
            after.objects_no_geometry,
        ),
        ("trees_loaded", before.trees_loaded, after.trees_loaded),
        (
            "asset_failures",
            before.asset_failures,
            after.asset_failures,
        ),
        (
            "scene_vertices",
            before.scene_vertices,
            after.scene_vertices,
        ),
    ];
    pairs
        .into_iter()
        .filter(|(_, a, b)| a != b)
        .map(|(k, a, b)| (k.to_string(), b as i64 - a as i64))
        .collect()
}

/// Where the tile places the object and where the analysis expects it (they differ only for
/// seeded faults).
fn placements(fault: Option<&str>) -> (mapgen::Placement, Placement) {
    let mut tile = mapgen::Placement::default();
    let mut expect = Placement {
        position: OBJECT_POS,
        heading_deg: OBJECT_HEADING_DEG,
        mirror_x: false,
        scale: 1.0,
    };
    match fault {
        Some("heading-mirror") => tile.heading_deg = -OBJECT_HEADING_DEG,
        Some("offset-10m") => tile.position.x += 10.0,
        Some("scale-2x") => expect.scale = 0.5,
        _ => {}
    }
    (tile, expect)
}

/// The views, framed on the expected geometry (a 20 m box when there is none).
fn frame(exp: &Expected, placement: &Placement) -> (Bounds, [View; 2]) {
    let placed = exp
        .tris
        .iter()
        .flat_map(|t| t.v)
        .map(|v| placement.to_map(v));
    let bounds = Bounds::of(placed).unwrap_or_else(|| Bounds {
        min: placement.position + Vec3::new(-10.0, -10.0, 0.0),
        max: placement.position + Vec3::new(10.0, 10.0, 20.0),
    });
    let views = [
        oblique_view(&bounds, placement.heading_deg),
        top_view(&bounds),
    ];
    (bounds, views)
}

/// Processes one object; the flag is `false` when the worker should be recycled afterwards.
pub fn process(h: &mut Harness, entry: &ObjectEntry) -> (Record, bool) {
    let t0 = Instant::now();
    let mut timing = Timing::default();
    let pre = preflight::run(&h.install, entry);
    let (tile_place, expect_place) = placements(h.config.seed_fault.as_deref());
    let strings_a = pre.strings.strings('A');
    let mut record = base_record(entry, &pre, h.config.id);
    timing.preflight = ms(t0);

    let t = Instant::now();
    let before = h.stats();
    h.drain_events();
    if let Err(e) = h.place(Some((&entry.sco, &strings_a, tile_place))) {
        tracing::error!(sco = %entry.sco, "cannot place object: {e:#}");
        record.categories.push("F-LOAD-ERROR".into());
        record.verdict = super::Verdict::Fail;
        return (record, true);
    }
    timing.load = ms(t);
    let t = Instant::now();
    let deadline = t0 + h.config.object_timeout;
    let settled = h.settle(deadline).unwrap_or(false);
    timing.settle = ms(t);
    let drained = h.drain_events();
    let events = h.baseline.filter(drained);
    let stats = stats_delta(&before, &h.stats());
    if !settled {
        record.categories.push("F-TIMEOUT".into());
        record.verdict = super::Verdict::Fail;
        record.diagnostics = super::Diagnostics { stats, events };
        timing.total = ms(t0);
        record.timing_ms = timing;
        return (record, false);
    }

    let t = Instant::now();
    let exp = match (&pre.object, &pre.path) {
        (Some(o), Some(p)) => expected::build(&h.install, p, o),
        _ => Expected::default(),
    };
    timing.preflight += ms(t);
    let (bounds, views) = frame(&exp, &expect_place);
    let t = Instant::now();
    let pixels = h.capture(&views);
    timing.render = ms(t);
    let Ok(pixels) = pixels else {
        record.categories.push("F-CAPTURE".into());
        record.verdict = super::Verdict::Fail;
        record.diagnostics = super::Diagnostics { stats, events };
        return (record, false);
    };

    let t = Instant::now();
    let mut analysis = analyse(&pixels, &exp, &expect_place, &views);
    record.views = analysis.metrics.clone();
    timing.analyse = ms(t);
    let text = text_check(h, entry, &pre, &exp, &expect_place, tile_place).map(|(c, shots)| {
        analysis.text_shots = shots;
        c
    });

    let diagnostics = super::Diagnostics { stats, events };
    judge(h, &mut record, &pre, &exp, diagnostics, text);
    let editor_only = record.class.iter().any(|c| c == "editor-only")
        && !record.class.iter().any(|c| c == "tree");
    if h.config.scripts && script_check::is_scripted(&pre) && editor_only {
        // The loader spawns nothing for `[onlyeditor]` objects: there is no script to run.
        record.tags.script = Some("editor-only".into());
    } else if h.config.scripts && script_check::is_scripted(&pre) {
        let script_views = script_check::views(&bounds, expect_place.heading_deg, &pre, &views);
        let thresholds = h.config.thresholds.script.clone();
        let ctx = script_run::Ctx {
            entry,
            pre: &pre,
            strings: &strings_a,
            tile_place,
            views: &script_views,
            deadline: t0 + h.config.object_timeout,
            thresholds: &thresholds,
            effects_off: h.config.seed_fault.as_deref() == Some("script-effect-off"),
        };
        analysis.text_shots.extend(script_check::stage(
            h,
            &ctx,
            &exp,
            &expect_place,
            &mut record,
        ));
    }
    images::keep(h, &mut record, &analysis);
    timing.total = ms(t0);
    record.timing_ms = timing;
    (record, true)
}

/// Combines load-stage evidence and metrics into the verdict.
fn judge(
    h: &Harness,
    record: &mut Record,
    pre: &Preflight,
    exp: &Expected,
    diagnostics: super::Diagnostics,
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

fn base_record(entry: &ObjectEntry, pre: &Preflight, worker: usize) -> Record {
    let (status, channels) = script_check::evaluate(pre);
    let sound = pre.object.as_ref().and_then(|o| o.sound.clone());
    let resolves = sound.as_ref().map(|s| {
        pre.path
            .as_ref()
            .and_then(|p| p.parent())
            .is_some_and(|d| d.join(s.replace('\\', "/")).exists())
    });
    Record {
        sco: entry.sco.clone(),
        key: entry.key.clone(),
        tier: entry.tier.clone(),
        class: pre.classes.clone(),
        tags: Tags {
            unsupported: pre.unsupported(),
            script: status,
            effect_channels: channels,
            info: Vec::new(),
        },
        audio: Audio {
            status: "recorded".into(),
            sound_cfg: sound,
            resolves,
            triggers: Vec::new(),
        },
        worker,
        ..Record::default()
    }
}

fn no_expected_reason(pre: &Preflight, exp: &Expected) -> String {
    if exp.keyed.unresolved > 0 {
        "keyed mesh the generator does not cover".into()
    } else if pre.classes.iter().any(|c| c == "vehicle-model") {
        "[model] objects are not predicted".into()
    } else {
        "no expected silhouette".into()
    }
}

/// Mattes, masks and metrics of both views.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// Per-view mattes (`v1`, `v2`).
    pub mattes: Vec<Matte>,
    /// Per-view expected masks.
    pub expected: Vec<ExpectedMasks>,
    /// Per-view metrics.
    pub metrics: BTreeMap<String, super::ViewMetrics>,
    /// Text close-ups (`t1_a`, `t1_b`, `t2_a`, `t2_b`), kept as images on request.
    pub text_shots: Vec<(String, Matte)>,
}

fn analyse(px: &[ViewPixels; 2], exp: &Expected, place: &Placement, views: &[View; 2]) -> Analysis {
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
fn text_check(
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
