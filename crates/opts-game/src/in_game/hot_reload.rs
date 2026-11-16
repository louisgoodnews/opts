//! The game side of hot reload: applies validated file changes to the running world, keeps the
//! watcher pointed at the directories the session loaded from, and restarts the session when the
//! player vehicle changed. Pipeline and validation live in `opts_bevy::hot_reload`.

use bevy::prelude::*;
use opts_bevy::control::state::Waiter;
use opts_bevy::hot_reload::{FileWatcher, ReloadApplierRegistered, ReloadKind};

use crate::control::restart_waiter;
use crate::in_game::omsi_world::{ReloadPlan, WorldLoadJob};

mod apply;
pub use apply::apply_reloads;

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
    plan.apply(world);
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
    *since += time.delta_secs();
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
