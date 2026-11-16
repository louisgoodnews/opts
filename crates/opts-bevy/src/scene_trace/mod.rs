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

pub use filter::{Filters, Radius};
pub use store::{Status, TileSummary};
pub use types::*;

mod access;
mod mode;
mod notes;

pub use access::*;
pub use mode::*;
pub use notes::*;

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


#[cfg(test)]
mod tests;
