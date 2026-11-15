//! Reading the trace: run status, queries, records by id and per-tile summaries.

use super::*;

/// Run status for `trace.set`.
#[must_use]
pub fn status() -> Status {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(store::Store::status))
        .unwrap_or_default()
}

/// Records matching a query, newest tile order preserved, at most `limit`.
#[must_use]
pub fn query(q: &Query) -> Vec<TraceRecord> {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.query(q)))
        .unwrap_or_default()
}

/// The record with `id`.
#[must_use]
pub fn get(id: u64) -> Option<TraceRecord> {
    STATE
        .lock()
        .ok()?
        .as_ref()?
        .records
        .iter()
        .find(|r| r.id == id)
        .cloned()
}

/// Per-tile counts.
#[must_use]
pub fn tile_summary(key: (i32, i32)) -> Option<TileSummary> {
    STATE.lock().ok()?.as_ref()?.summary_of(key)
}

/// Summaries of all tiles seen.
#[must_use]
pub fn summaries() -> Vec<TileSummary> {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(store::Store::summaries))
        .unwrap_or_default()
}

/// Every in-memory record that matches, as JSON Lines (for `trace.export`).
#[must_use]
pub fn export_lines(q: &Query) -> Vec<String> {
    query(q)
        .iter()
        .filter_map(|r| serde_json::to_string(r).ok())
        .collect()
}

/// A snapshot of records for the overlay (id, kind, quality, transform, bounds); bounded.
#[must_use]
pub fn overlay_snapshot(center: [f32; 2], radius: f32, limit: usize) -> Vec<TraceRecord> {
    query(&Query {
        near: Some((center, radius)),
        limit,
        ..Query::default()
    })
}
