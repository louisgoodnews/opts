//! The script check's inputs for one object and its stage.

use super::*;

/// What one object's script check works from.
pub(super) struct ScriptJob<'a> {
    pub(super) entry: &'a ObjectEntry,
    pub(super) pre: &'a Preflight,
    pub(super) strings_a: &'a [String],
    pub(super) tile_place: mapgen::Placement,
    pub(super) bounds: &'a Bounds,
    pub(super) views: &'a [View; 2],
    pub(super) expect_place: &'a Placement,
    pub(super) exp: &'a Expected,
    pub(super) t0: Instant,
}

pub(super) fn run_scripts(
    h: &mut Harness,
    job: &ScriptJob,
    record: &mut Record,
    analysis: &mut Analysis,
) {
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
