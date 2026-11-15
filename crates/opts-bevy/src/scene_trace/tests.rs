//! Unit tests of `scene_trace`: the helpers here, the tests in the topic modules.

use std::path::Path;
use std::sync::Mutex;

use bevy::math::Vec3;

use super::*;

/// The trace is process-global state: tests that touch it run one at a time.
static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "opts-scene-trace-test-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn rec(kind: &str, tile: (i32, i32), file: &str, at: [f32; 3]) -> (TraceRecord, [f32; 3]) {
    (
        TraceRecord {
            kind: kind.into(),
            tile: Some([tile.0, tile.1]),
            file: file.into(),
            world: Some(TraceTransform {
                t: at,
                q: [0.0, 0.0, 0.0, 1.0],
                s: [1.0; 3],
            }),
            bounds: Some(TraceBounds {
                min: [-1.0; 3],
                max: [1.0; 3],
            }),
            ..TraceRecord::default()
        },
        at,
    )
}

fn put(kind: &str, tile: (i32, i32), file: &str, at: [f32; 3]) -> Option<u64> {
    let (r, w) = rec(kind, tile, file, at);
    record(
        &Probe {
            kind,
            tile: Some(tile),
            file,
            world: w,
        },
        || r,
    )
}

mod filters;
mod jsonl;
mod modes;
mod parse;
mod query;
