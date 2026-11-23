//! Hot reload of OMSI content (`docs/src/architecture/hot-reload.md`, ADR-0007).
//!
//! Pipeline: [`FileWatcher`] → [`Debouncer`] → [`ReloadRequested`] → validation on the task pool →
//! [`ReloadValidated`] or [`ReloadFailed`]. Applying the change (rebuilding tiles, textures, the
//! vehicle) is the application's job: it reads [`ReloadValidated`] and writes [`ReloadApplied`] or
//! [`ReloadFailed`]. A file that does not parse never reaches the apply step, so a half-saved file
//! cannot remove working content.

pub mod cache;
/// The file-event debouncer, now the `opts-debounce` crate (same paths).
pub use opts_debounce as debounce;
pub mod events;
pub mod validate;
pub mod watcher;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

pub use debounce::Debouncer;
pub use events::{
    classify, is_editor_scratch, ReloadApplied, ReloadApplierRegistered, ReloadFailed, ReloadKind,
    ReloadOutcome, ReloadRequested, ReloadSceneRequested, ReloadStats, ReloadValidated,
};
pub use watcher::FileWatcher;

/// Settings of the hot-reload plugin.
#[derive(Resource, Debug, Clone)]
pub struct HotReloadConfig {
    /// Whether the file watcher runs at all.
    pub enabled: bool,
    /// Quiet period that coalesces bursts of file events.
    pub debounce: Duration,
    /// Directories watched from the start (recursively).
    pub roots: Vec<PathBuf>,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce: Duration::from_millis(250),
            roots: Vec::new(),
        }
    }
}

#[derive(Resource)]
struct PendingDebounce(Debouncer);

struct Validation {
    path: PathBuf,
    kind: ReloadKind,
    seq: u64,
    task: Task<Result<(), String>>,
}

#[derive(Resource, Default)]
struct Validations(Vec<Validation>);

/// Adds the watcher, the debounce stage and validation.
#[derive(Default)]
pub struct HotReloadPlugin {
    /// Settings.
    pub config: HotReloadConfig,
}

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .insert_resource(PendingDebounce(Debouncer::new(self.config.debounce)))
            .init_resource::<Validations>()
            .init_resource::<ReloadStats>()
            .add_message::<ReloadRequested>()
            .add_message::<ReloadValidated>()
            .add_message::<ReloadFailed>()
            .add_message::<ReloadApplied>()
            .add_message::<ReloadSceneRequested>();
        if self.config.enabled {
            match FileWatcher::new() {
                Ok(w) => {
                    for root in &self.config.roots {
                        if let Err(e) = w.add(root, true) {
                            tracing::warn!(target: "opts_bevy::hot_reload", root = %root.display(), "cannot watch: {e}");
                        }
                    }
                    app.insert_resource(w);
                }
                Err(e) => {
                    tracing::warn!(target: "opts_bevy::hot_reload", "file watching is unavailable: {e}")
                }
            }
        }
        app.add_systems(
            Update,
            (
                collect_file_events,
                start_validations,
                finish_validations,
                record_outcomes,
            )
                .chain(),
        );
    }
}
mod systems;

use systems::*;
