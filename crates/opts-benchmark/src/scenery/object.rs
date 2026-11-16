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
use base_record::base_record;
use diagnostics::settle_object;
use judging::judge;
use placement::{frame, placements};
use script_job::{run_scripts, ScriptJob};

mod analysis;
pub use analysis::Analysis;
use analysis::{analyse, text_check};

mod base_record;
mod diagnostics;
mod judging;
mod placement;
mod script_job;

/// Milliseconds since `t`.
fn ms(t: Instant) -> u64 {
    t.elapsed().as_millis() as u64
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
