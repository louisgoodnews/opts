//! Scene trace (M-0087): which mesh is placed where.
//!
//! An opt-in debug mode that keeps one [`TraceRecord`] per placed thing (scenery object, tree,
//! parked vehicle, spline, terrain tile, traffic car, pedestrian, session vehicle) and writes
//! them as JSON Lines next to the run logs. The records answer the question *which source file is
//! this defect?* for people and for agents (control protocol `trace.*`).
//!
//! The mode is `off` (default), `summary` (per-tile counts and issues only) or `full` (every
//! record). **Off is free:** the hooks test one atomic ([`enabled`]); no record, string or
//! collection is built, and the state below is not even allocated (test `off_allocates_nothing`).
//!
//! See `docs/src/architecture/scene-trace.md`.

pub mod filter;
pub mod pick;
mod store;
mod types;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

pub use filter::{Filters, Radius};
pub use store::{Status, TileSummary};
pub use types::*;

/// How much the trace keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Nothing is kept (default).
    #[default]
    Off,
    /// Counts per tile and kind, and a record for every thing with an issue.
    Summary,
    /// A record for every placed thing.
    Full,
}

impl Mode {
    /// Parses `off`, `summary` or `full` (case-insensitive); the empty text is `off`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "off" | "false" | "0" | "no" => Some(Self::Off),
            "summary" | "sum" | "true" | "1" | "on" => Some(Self::Summary),
            "full" | "all" | "2" => Some(Self::Full),
            _ => None,
        }
    }

    /// The text form (`off`, `summary`, `full`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Summary => "summary",
            Self::Full => "full",
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Summary => 1,
            Self::Full => 2,
        }
    }

    const fn from_code(c: u8) -> Self {
        match c {
            1 => Self::Summary,
            2 => Self::Full,
            _ => Self::Off,
        }
    }
}

static MODE: AtomicU8 = AtomicU8::new(0);
static STATE: Mutex<Option<store::Store>> = Mutex::new(None);

/// The current mode.
#[must_use]
pub fn mode() -> Mode {
    Mode::from_code(MODE.load(Ordering::Relaxed))
}

/// Whether the trace is on. The one check every hook makes first.
#[inline]
#[must_use]
pub fn enabled() -> bool {
    MODE.load(Ordering::Relaxed) != 0
}

/// Whether the trace state is allocated (false while off; used by the zero-cost test).
#[must_use]
pub fn state_allocated() -> bool {
    STATE.lock().map(|s| s.is_some()).unwrap_or(false)
}

/// Sets the directory the JSONL files go to (the game passes `<log directory>/trace`).
pub fn set_directory(dir: PathBuf) {
    store::set_directory(dir);
}

/// Sets the mode, allocating or dropping the state. Returns the previous mode.
pub fn set_mode(mode: Mode) -> Mode {
    let previous = self::mode();
    if mode == previous {
        return previous;
    }
    if let Ok(mut state) = STATE.lock() {
        if mode == Mode::Off {
            if let Some(s) = state.as_mut() {
                s.close("mode off");
            }
            *state = None;
        } else {
            let s = state.get_or_insert_with(store::Store::new);
            s.mode = mode;
            s.open_file_if_ready();
        }
    }
    MODE.store(mode.code(), Ordering::Relaxed);
    tracing::info!(mode = mode.name(), "scene trace mode");
    previous
}

/// Replaces the filters (the state must be on).
pub fn set_filters(filters: Filters) {
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.filters = filters;
        }
    }
}

/// The current filters.
#[must_use]
pub fn filters() -> Filters {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.filters.clone()))
        .unwrap_or_default()
}

/// Sets the size cap of the JSONL file in bytes (default 256 MiB).
pub fn set_size_cap(bytes: u64) {
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.size_cap = bytes;
        }
    }
}

/// A map session starts: names the run (the header record) and, when the trace is on, opens a
/// fresh file. `root` is the OMSI folder paths are made relative to.
pub fn begin_session(map: &str, root: &Path) {
    if !enabled() {
        // Remember for a later `set_mode`.
        store::remember_session(map, root);
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.begin_session(map, root);
        }
    }
}

/// A tile starts loading: earlier records of the same tile (a rebuild) are superseded.
pub fn begin_tile(key: (i32, i32)) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.begin_tile(key);
        }
    }
}

/// A tile has been loaded: summary mode writes its counts line.
pub fn end_tile(key: (i32, i32)) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.end_tile(key);
        }
    }
}

/// A tile was unloaded: its records leave the in-memory index (the file keeps them).
pub fn forget_tile(key: (i32, i32)) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.forget_tile(key);
        }
    }
}

/// Whether a thing passes the filters (build the record only when it does).
#[must_use]
pub fn wants(probe: &Probe<'_>) -> bool {
    if !enabled() {
        return false;
    }
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.filters.accepts(probe)))
        .unwrap_or(false)
}

/// Records a thing. The closure runs only when the trace is on and the probe passes the filters,
/// so building the record costs nothing otherwise. Returns the record id.
pub fn record(probe: &Probe<'_>, build: impl FnOnce() -> TraceRecord) -> Option<u64> {
    if !wants(probe) {
        return None;
    }
    let mut rec = build();
    let mut state = STATE.lock().ok()?;
    let s = state.as_mut()?;
    Some(s.add(&mut rec))
}

/// Stores what the loader learned about a mesh file; later records pick it up with [`note_for`].
pub fn note_mesh(path: &Path, mut note: MeshNote) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            note.file = s.relative(path);
            s.notes.insert(store::norm(path), note);
        }
    }
}

/// Whether a note is stored for the file.
#[must_use]
pub fn has_note(path: &Path) -> bool {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.notes.contains_key(&store::norm(path))))
        .unwrap_or(false)
}

/// The note stored for a mesh file, or a bare `plain` note when the file was served from the
/// loader's caches without a fresh load.
#[must_use]
pub fn note_for(path: &Path) -> MeshNote {
    let Ok(state) = STATE.lock() else {
        return MeshNote::default();
    };
    let Some(s) = state.as_ref() else {
        return MeshNote::default();
    };
    s.notes
        .get(&store::norm(path))
        .cloned()
        .unwrap_or_else(|| MeshNote {
            file: s.relative(path),
            stage: "unknown".into(),
            ..MeshNote::default()
        })
}

/// A path relative to the OMSI folder (privacy: never an absolute path).
#[must_use]
pub fn relative(path: &Path) -> String {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.relative(path)))
        .unwrap_or_else(|| store::file_name_of(path))
}

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

#[cfg(test)]
mod tests;
