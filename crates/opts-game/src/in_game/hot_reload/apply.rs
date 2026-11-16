//! The apply system of the game's hot reload: plans each validated change against the running session
//! and reports what it did.

use std::path::PathBuf;
use std::time::Instant;

use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use opts_bevy::hot_reload::{
    ReloadApplied, ReloadFailed, ReloadKind, ReloadSceneRequested, ReloadValidated,
};

use super::{execute, plan};
use crate::in_game::omsi_world::WorldLoadJob;

/// Consumes [`ReloadValidated`] and [`ReloadSceneRequested`].
pub fn apply_reloads(
    world: &mut World,
    mut validated: Local<MessageCursor<ReloadValidated>>,
    mut scenes: Local<MessageCursor<ReloadSceneRequested>>,
) {
    let files: Vec<ReloadValidated> = validated
        .read(world.resource::<Messages<ReloadValidated>>())
        .cloned()
        .collect();
    let scene_requests: Vec<ReloadSceneRequested> = scenes
        .read(world.resource::<Messages<ReloadSceneRequested>>())
        .copied()
        .collect();
    for ev in files {
        let started = Instant::now();
        match plan(world, &ev.path, ev.kind) {
            None => {
                world.write_message(ReloadApplied {
                    path: ev.path,
                    kind: ev.kind,
                    seq: ev.seq,
                    summary: "no session is loaded".into(),
                });
            }
            Some(Ok(p)) => {
                execute(world, &p);
                let summary = format!("{} ({} ms)", p.summary(), started.elapsed().as_millis());
                world.write_message(ReloadApplied {
                    path: ev.path,
                    kind: ev.kind,
                    seq: ev.seq,
                    summary,
                });
            }
            Some(Err(error)) => {
                tracing::warn!(target: "opts_bevy::hot_reload", path = %ev.path.display(), "reload failed, keeping the previous content: {error}");
                world.write_message(ReloadFailed {
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
            world.write_message(ReloadFailed {
                path: PathBuf::new(),
                kind: ReloadKind::Tile,
                seq: req.seq,
                error: "no session is loaded".into(),
            });
            continue;
        };
        execute(world, &p);
        world.write_message(ReloadApplied {
            path: PathBuf::new(),
            kind: ReloadKind::Tile,
            seq: req.seq,
            summary: p.summary(),
        });
    }
}
