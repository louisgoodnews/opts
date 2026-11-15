//! Load diagnostics of one object: `WorldStats` deltas and the settle wait.

use super::*;

/// The `WorldStats` counters that change while an object loads, as deltas.
pub(super) fn stats_delta(before: &WorldStats, after: &WorldStats) -> BTreeMap<String, i64> {
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

pub(super) fn settle_object(
    h: &mut Harness,
    t0: Instant,
    before: &WorldStats,
    timing: &mut Timing,
) -> (bool, super::super::Diagnostics) {
    let t = Instant::now();
    let deadline = t0 + h.config.object_timeout;
    let settled = h.settle(deadline).unwrap_or(false);
    timing.settle = ms(t);
    let drained = h.drain_events();
    let events = h.baseline.filter(drained);
    let stats = stats_delta(before, &h.stats());
    (settled, super::super::Diagnostics { stats, events })
}
