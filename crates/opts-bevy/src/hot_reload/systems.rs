//! Applying a reload: collecting the file events, starting and finishing the validations, and recording outcomes.

use super::*;

/// Moves watcher events through the debouncer and emits one [`ReloadRequested`] per quiet file.
pub(super) fn collect_file_events(
    watcher: Option<Res<FileWatcher>>,
    config: Res<HotReloadConfig>,
    mut debounce: ResMut<PendingDebounce>,
    mut stats: ResMut<ReloadStats>,
    mut out: MessageWriter<ReloadRequested>,
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
            out.write(ReloadRequested { path, kind, seq });
        }
    }
}

pub(super) fn start_validations(
    mut events: MessageReader<ReloadRequested>,
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

pub(super) fn finish_validations(
    mut tasks: ResMut<Validations>,
    mut ok: MessageWriter<ReloadValidated>,
    mut failed: MessageWriter<ReloadFailed>,
) {
    let mut still = Vec::new();
    for mut v in std::mem::take(&mut tasks.0) {
        match block_on(future::poll_once(&mut v.task)) {
            None => still.push(v),
            Some(Ok(())) => {
                ok.write(ReloadValidated {
                    path: v.path,
                    kind: v.kind,
                    seq: v.seq,
                });
            }
            Some(Err(error)) => {
                tracing::warn!(target: "opts_bevy::hot_reload", path = %v.path.display(), "reload failed, keeping the previous content: {error}");
                failed.write(ReloadFailed {
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
pub(super) fn record_outcomes(
    mut applied: MessageReader<ReloadApplied>,
    mut failed: MessageReader<ReloadFailed>,
    mut validated: MessageReader<ReloadValidated>,
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
