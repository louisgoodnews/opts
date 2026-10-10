//! Hot reload of OMSI content (`docs/src/architecture/hot-reload.md`, ADR-0007).
//!
//! Pipeline: [`FileWatcher`] → [`Debouncer`] → [`ReloadRequested`] → validation on the task pool →
//! [`ReloadValidated`] or [`ReloadFailed`]. Applying the change (rebuilding tiles, textures, the
//! vehicle) is the application's job: it reads [`ReloadValidated`] and writes [`ReloadApplied`] or
//! [`ReloadFailed`]. A file that does not parse never reaches the apply step, so a half-saved file
//! cannot remove working content.

pub mod cache;
pub mod debounce;
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
            .add_event::<ReloadRequested>()
            .add_event::<ReloadValidated>()
            .add_event::<ReloadFailed>()
            .add_event::<ReloadApplied>()
            .add_event::<ReloadSceneRequested>();
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

/// Moves watcher events through the debouncer and emits one [`ReloadRequested`] per quiet file.
fn collect_file_events(
    watcher: Option<Res<FileWatcher>>,
    config: Res<HotReloadConfig>,
    mut debounce: ResMut<PendingDebounce>,
    mut stats: ResMut<ReloadStats>,
    mut out: EventWriter<ReloadRequested>,
) {
    debounce.0.set_window(config.debounce);
    let now = Instant::now();
    if let Some(w) = watcher {
        for path in w.poll() {
            if classify(&path).is_some() {
                debounce.0.push(path, now);
            }
        }
    }
    for path in debounce.0.drain_ready(now) {
        if let Some(kind) = classify(&path) {
            let seq = stats.allocate();
            tracing::info!(target: "opts_bevy::hot_reload", path = %path.display(), kind = kind.label(), "reload requested (file changed)");
            out.send(ReloadRequested { path, kind, seq });
        }
    }
}

fn start_validations(
    mut events: EventReader<ReloadRequested>,
    mut tasks: ResMut<Validations>,
    mut stats: ResMut<ReloadStats>,
) {
    for ev in events.read() {
        stats.requested += 1;
        let (path, kind) = (ev.path.clone(), ev.kind);
        let task = AsyncComputeTaskPool::get().spawn({
            let path = path.clone();
            async move { validate::validate(&path, kind) }
        });
        tasks.0.push(Validation {
            path,
            kind,
            seq: ev.seq,
            task,
        });
    }
}

fn finish_validations(
    mut tasks: ResMut<Validations>,
    mut ok: EventWriter<ReloadValidated>,
    mut failed: EventWriter<ReloadFailed>,
) {
    let mut still = Vec::new();
    for mut v in std::mem::take(&mut tasks.0) {
        match block_on(future::poll_once(&mut v.task)) {
            None => still.push(v),
            Some(Ok(())) => {
                ok.send(ReloadValidated {
                    path: v.path,
                    kind: v.kind,
                    seq: v.seq,
                });
            }
            Some(Err(error)) => {
                tracing::warn!(target: "opts_bevy::hot_reload", path = %v.path.display(), "reload failed, keeping the previous content: {error}");
                failed.send(ReloadFailed {
                    path: v.path,
                    kind: v.kind,
                    seq: v.seq,
                    error,
                });
            }
        }
    }
    tasks.0 = still;
}

/// Folds applier answers and validation failures into [`ReloadStats`]. Without an applier a
/// validated file counts as finished.
fn record_outcomes(
    mut applied: EventReader<ReloadApplied>,
    mut failed: EventReader<ReloadFailed>,
    mut validated: EventReader<ReloadValidated>,
    has_applier: Option<Res<ReloadApplierRegistered>>,
    mut stats: ResMut<ReloadStats>,
) {
    for a in applied.read() {
        tracing::info!(target: "opts_bevy::hot_reload", path = %a.path.display(), kind = a.kind.label(), summary = %a.summary, "reload applied");
        stats.record(ReloadOutcome {
            seq: a.seq,
            path: a.path.clone(),
            result: Ok(a.summary.clone()),
        });
    }
    for f in failed.read() {
        stats.record(ReloadOutcome {
            seq: f.seq,
            path: f.path.clone(),
            result: Err(f.error.clone()),
        });
    }
    if has_applier.is_none() {
        for v in validated.read() {
            stats.record(ReloadOutcome {
                seq: v.seq,
                path: v.path.clone(),
                result: Ok("validated (no applier)".into()),
            });
        }
    }
}
