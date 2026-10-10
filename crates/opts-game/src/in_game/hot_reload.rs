//! The game side of hot reload: applies validated file changes to the running world, keeps the
//! watcher pointed at the directories the session loaded from, and restarts the session when the
//! player vehicle changed. Pipeline and validation live in `opts_bevy::hot_reload`.

use std::path::PathBuf;
use std::time::Instant;

use bevy::ecs::event::{Events, ManualEventReader};
use bevy::prelude::*;
use opts_bevy::control::state::Waiter;
use opts_bevy::hot_reload::{
    FileWatcher, ReloadApplied, ReloadApplierRegistered, ReloadFailed, ReloadKind,
    ReloadSceneRequested, ReloadValidated,
};

use crate::control::restart_waiter;
use crate::in_game::omsi_world::{ReloadPlan, WorldLoadJob};

/// A session restart started by a vehicle change; polled until the new session is up.
#[derive(Resource, Default)]
pub struct ActiveRestart(std::sync::Mutex<Option<Waiter>>);

/// Registers the apply systems.
pub struct GameHotReloadPlugin;

impl Plugin for GameHotReloadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReloadApplierRegistered)
            .init_resource::<ActiveRestart>()
            .add_systems(Update, (apply_reloads, drive_restart, sync_watch_roots));
    }
}

fn execute(world: &mut World, plan: &ReloadPlan) {
    for e in &plan.despawn {
        if let Some(entity) = world.get_entity_mut(*e) {
            entity.despawn_recursive();
        }
    }
    if !plan.restart_vehicle {
        return;
    }
    let idle = world
        .resource::<ActiveRestart>()
        .0
        .lock()
        .is_ok_and(|g| g.is_none());
    if idle {
        let waiter = restart_waiter(world);
        if let Ok(mut g) = world.resource::<ActiveRestart>().0.lock() {
            *g = Some(waiter);
        }
    }
}

/// Plans a validated change against the world job; `None` when no session is loaded.
fn plan(
    world: &mut World,
    path: &std::path::Path,
    kind: ReloadKind,
) -> Option<Result<ReloadPlan, String>> {
    world.get_resource::<WorldLoadJob>()?;
    Some(world.resource_scope(|world, mut job: Mut<WorldLoadJob>| {
        world.resource_scope(|_, mut images: Mut<Assets<Image>>| {
            job.plan_reload(path, kind, &mut images)
        })
    }))
}

/// Consumes [`ReloadValidated`] and [`ReloadSceneRequested`].
pub fn apply_reloads(
    world: &mut World,
    mut validated: Local<ManualEventReader<ReloadValidated>>,
    mut scenes: Local<ManualEventReader<ReloadSceneRequested>>,
) {
    let files: Vec<ReloadValidated> = validated
        .read(world.resource::<Events<ReloadValidated>>())
        .cloned()
        .collect();
    let scene_requests: Vec<ReloadSceneRequested> = scenes
        .read(world.resource::<Events<ReloadSceneRequested>>())
        .copied()
        .collect();
    for ev in files {
        let started = Instant::now();
        match plan(world, &ev.path, ev.kind) {
            None => {
                world.send_event(ReloadApplied {
                    path: ev.path,
                    kind: ev.kind,
                    seq: ev.seq,
                    summary: "no session is loaded".into(),
                });
            }
            Some(Ok(p)) => {
                execute(world, &p);
                let summary = format!("{} ({} ms)", p.summary(), started.elapsed().as_millis());
                world.send_event(ReloadApplied {
                    path: ev.path,
                    kind: ev.kind,
                    seq: ev.seq,
                    summary,
                });
            }
            Some(Err(error)) => {
                tracing::warn!(target: "opts_bevy::hot_reload", path = %ev.path.display(), "reload failed, keeping the previous content: {error}");
                world.send_event(ReloadFailed {
                    path: ev.path,
                    kind: ev.kind,
                    seq: ev.seq,
                    error,
                });
            }
        }
    }
    for req in scene_requests {
        let planned = world
            .get_resource::<WorldLoadJob>()
            .is_some()
            .then(|| world.resource_scope(|_, mut job: Mut<WorldLoadJob>| job.plan_scene_reload()));
        let Some(p) = planned else {
            world.send_event(ReloadFailed {
                path: PathBuf::new(),
                kind: ReloadKind::Tile,
                seq: req.seq,
                error: "no session is loaded".into(),
            });
            continue;
        };
        execute(world, &p);
        world.send_event(ReloadApplied {
            path: PathBuf::new(),
            kind: ReloadKind::Tile,
            seq: req.seq,
            summary: p.summary(),
        });
    }
}

fn drive_restart(world: &mut World) {
    let taken = world
        .resource::<ActiveRestart>()
        .0
        .lock()
        .ok()
        .and_then(|mut g| g.take());
    let Some(mut waiter) = taken else { return };
    if waiter(world).is_none() {
        if let Ok(mut g) = world.resource::<ActiveRestart>().0.lock() {
            *g = Some(waiter);
        }
    } else {
        tracing::info!(target: "opts_bevy::hot_reload", "session restarted after a vehicle change");
    }
}

/// Once a second: watches the map folder and every directory content was loaded from.
fn sync_watch_roots(
    time: Res<Time<Real>>,
    watcher: Option<Res<FileWatcher>>,
    job: Option<Res<WorldLoadJob>>,
    mut since: Local<f32>,
) {
    *since += time.delta_seconds();
    if *since < 1.0 {
        return;
    }
    *since = 0.0;
    let (Some(watcher), Some(job)) = (watcher, job) else {
        return;
    };
    let map_dir = opts_bevy::hot_reload::cache::normalize(job.map_dir());
    if !watcher.is_watching(&map_dir) {
        let _ = watcher.add(&map_dir, true);
    }
    let mut added = 0;
    for dir in job.watch_dirs() {
        if !watcher.is_watching(&dir)
            && !dir.starts_with(&map_dir)
            && watcher.add(&dir, false).unwrap_or(false)
        {
            added += 1;
        }
    }
    if added > 0 {
        tracing::debug!(target: "opts_bevy::hot_reload", added, total = watcher.roots().len(), "watching more directories");
    }
}
