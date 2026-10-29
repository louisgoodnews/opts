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

mod analysis;
pub use analysis::Analysis;
use analysis::{analyse, text_check};

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

fn settle_object(
    h: &mut Harness,
    t0: Instant,
    before: &WorldStats,
    timing: &mut Timing,
) -> (bool, super::Diagnostics) {
    let t = Instant::now();
    let deadline = t0 + h.config.object_timeout;
    let settled = h.settle(deadline).unwrap_or(false);
    timing.settle = ms(t);
    let drained = h.drain_events();
    let events = h.baseline.filter(drained);
    let stats = stats_delta(before, &h.stats());
    (settled, super::Diagnostics { stats, events })
}

/// What one object's script check works from.
struct ScriptJob<'a> {
    entry: &'a ObjectEntry,
    pre: &'a Preflight,
    strings_a: &'a [String],
    tile_place: mapgen::Placement,
    bounds: &'a Bounds,
    views: &'a [View; 2],
    expect_place: &'a Placement,
    exp: &'a Expected,
    t0: Instant,
}

fn run_scripts(h: &mut Harness, job: &ScriptJob, record: &mut Record, analysis: &mut Analysis) {
    let ScriptJob {
        entry,
        pre,
        strings_a,
        tile_place,
        bounds,
        views,
        expect_place,
        exp,
        t0,
    } = *job;
    let editor_only = record.class.iter().any(|c| c == "editor-only")
        && !record.class.iter().any(|c| c == "tree");
    if h.config.scripts && script_check::is_scripted(pre) && editor_only {
        record.tags.script = Some("editor-only".into());
    } else if h.config.scripts && script_check::is_scripted(pre) {
        let script_views = script_check::views(bounds, expect_place.heading_deg, pre, views);
        let thresholds = h.config.thresholds.script.clone();
        let ctx = script_run::Ctx {
            entry,
            pre,
            strings: strings_a,
            tile_place,
            views: &script_views,
            deadline: t0 + h.config.object_timeout,
            thresholds: &thresholds,
            effects_off: h.config.seed_fault.as_deref() == Some("script-effect-off"),
        };
        analysis
            .text_shots
            .extend(script_check::stage(h, &ctx, exp, expect_place, record));
    }
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

    let (settled, diagnostics) = settle_object(h, t0, &before, &mut timing);
    if !settled {
        record.categories.push("F-TIMEOUT".into());
        record.verdict = super::Verdict::Fail;
        record.diagnostics = diagnostics;
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
        record.diagnostics = diagnostics;
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

    judge(h, &mut record, &pre, &exp, diagnostics, text);
    let job = ScriptJob {
        entry,
        pre: &pre,
        strings_a: &strings_a,
        tile_place,
        bounds: &bounds,
        views: &views,
        expect_place: &expect_place,
        exp: &exp,
        t0,
    };
    run_scripts(h, &job, &mut record, &mut analysis);
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
