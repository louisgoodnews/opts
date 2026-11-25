//! Scene trace hooks (M-0087): turn what the loaders place into [`TraceRecord`]s.
//!
//! Everything here is behind `opts_bevy::scene_trace::enabled()`; with the trace off the hooks in
//! the loaders reduce to one atomic load. See `docs/src/architecture/scene-trace.md`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::scene_trace::{self as st, kind, Probe, TraceRecord, TraceTransform};
use opts_core::formats::map::{MapObjectInstance, MapTile};
use serde_json::json;

use super::{WorldLoadJob, WorldStats};

/// What the loaders learn while one thing is loaded (collected in a stack, because a parked-car
/// marker loads the car it stands for from inside its own load).
#[derive(Default)]
pub(super) struct Frame {
    /// Mesh files the thing used.
    pub meshes: Vec<PathBuf>,
    /// Problems seen.
    pub warnings: Vec<String>,
    /// The thing is not a placed thing of its own (a parked-car marker): no record.
    pub skip: bool,
    /// Overrides the record kind (`tree`, `parked`).
    pub kind: Option<&'static str>,
    /// World transform the loader computed.
    pub transform: Option<Transform>,
    /// `.cfg` of a vehicle-style model.
    pub model_cfg: Option<String>,
}

thread_local! {
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

mod records;
pub(super) use records::{human_frame, remember_human, spawned, tile_content};
mod site;
pub(super) use site::{notes, to_transform, Counters, Site};
mod object;
mod parents;
pub(in crate) use object::object;

/// Opens a frame (no-op while the trace is off).
pub(super) fn begin_frame() {
    if st::enabled() {
        FRAMES.with(|f| f.borrow_mut().push(Frame::default()));
    }
}

/// Closes the innermost frame.
pub(super) fn end_frame() -> Frame {
    if !st::enabled() {
        return Frame::default();
    }
    FRAMES.with(|f| f.borrow_mut().pop()).unwrap_or_default()
}

fn with_frame(f: impl FnOnce(&mut Frame)) {
    if st::enabled() {
        FRAMES.with(|fr| {
            if let Some(top) = fr.borrow_mut().last_mut() {
                f(top);
            }
        });
    }
}

/// A mesh file was loaded for the thing in progress.
pub(super) fn mesh_used(path: &Path) {
    with_frame(|f| {
        if !f.meshes.iter().any(|p| p == path) {
            f.meshes.push(path.to_path_buf());
        }
    });
}

/// A problem with the thing in progress.
pub(super) fn warn(text: impl FnOnce() -> String) {
    with_frame(|f| f.warnings.push(text()));
}

/// The thing in progress is not a placed thing of its own.
pub(super) fn skip_record() {
    with_frame(|f| f.skip = true);
}

/// Sets the kind of the thing in progress.
pub(super) fn set_kind(k: &'static str) {
    with_frame(|f| f.kind = Some(k));
}

/// The world transform of the thing in progress.
pub(super) fn set_transform(t: &Transform) {
    with_frame(|f| f.transform = Some(*t));
}

/// The vehicle-style model of the thing in progress.
pub(super) fn set_model_cfg(cfg: &str) {
    with_frame(|f| f.model_cfg = Some(cfg.to_owned()));
}
