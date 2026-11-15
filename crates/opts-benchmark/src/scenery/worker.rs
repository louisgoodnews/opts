//! The worker process: a windowless Bevy app built on `opts-game`'s map loader.
//!
//! It loads the generated map once with `begin_world_load`, then for every object rewrites the
//! tile, rebuilds it through the loader (`plan_scene_reload` + `load_next_tile`, no file watcher),
//! waits until the tile is stable, and captures the views. The per-object analysis is in [`super::object`].

use std::io::{BufRead, Write as _};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use bevy::app::SubApps;
use bevy::prelude::*;
use bevy::render::render_resource::PollType;
use bevy::render::renderer::RenderDevice;
use bevy::render::RenderPlugin;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use opts_bevy::{OmsiInstall, SceneryClock, SceneryLodMode, SceneryRuntimeSettings};
use opts_game::in_game::omsi_world::{
    begin_world_load, load_next_tile, OmsiSessionConfig, WorldLoadJob, WorldStats,
};

use super::capture::{CaptureRig, ViewPixels};
use super::diag::{Baseline, EventSink};
use super::framing::View;
use super::mapgen::{MapWriter, Placement};
use super::protocol::WorkerMsg;
use super::{lookups, ObjectEntry};

mod config;
mod frames;
mod harness;
mod loading;
mod readback;

pub use config::WorkerConfig;
pub use harness::Harness;

/// Loads every queued tile.
fn drive_tiles(
    mut job: ResMut<WorldLoadJob>,
    mut stats: ResMut<WorldStats>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    while !job.tiles_done() {
        load_next_tile(
            &mut job,
            &mut stats,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            None,
        );
    }
}

/// Runs the worker: announces readiness, then processes objects read from stdin.
///
/// # Errors
/// The harness cannot be built.
pub fn run(config: WorkerConfig, sink: EventSink) -> Result<()> {
    opts_bevy::texture_sentinel::set_missing_texture_sentinel(true);
    let mut h = Harness::new(config, sink)?;
    send(&WorkerMsg::Ready);
    let stdin = std::io::stdin();
    let mut processed = 0;
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: ObjectEntry =
            serde_json::from_str(&line).map_err(|e| anyhow!("job line: {e}"))?;
        send(&WorkerMsg::Start {
            key: entry.key.clone(),
        });
        inject_fault(&entry.sco);
        let (record, healthy) = super::object::process(&mut h, &entry);
        processed += 1;
        let recycle = if !healthy {
            Some("soft timeout: background work may still run".to_string())
        } else if processed >= h.config.max_objects {
            Some(format!("recycled after {processed} objects"))
        } else {
            None
        };
        // `Recycle` goes out before the result: the supervisor must know the worker is leaving
        // before it sees the worker idle, or it would hand the next object to a dying process.
        if let Some(reason) = &recycle {
            send(&WorkerMsg::Recycle {
                reason: reason.clone(),
            });
        }
        send(&WorkerMsg::Result {
            record: Box::new(record),
        });
        if recycle.is_some() {
            break;
        }
    }
    Ok(())
}

/// Test hook for the supervisor's crash and hang recovery: `OPTS_BENCH_INJECT` holds
/// `crash:<text>` or `hang:<text>`; an object whose path contains `<text>` aborts the worker or
/// blocks it forever. Unset in normal runs.
fn inject_fault(sco: &str) {
    let Ok(spec) = std::env::var("OPTS_BENCH_INJECT") else {
        return;
    };
    let Some((kind, text)) = spec.split_once(':') else {
        return;
    };
    if !sco.to_lowercase().contains(&text.to_lowercase()) {
        return;
    }
    match kind {
        "crash" => std::process::abort(),
        "hang" => loop {
            std::thread::sleep(Duration::from_secs(3600));
        },
        _ => {}
    }
}

fn send(msg: &WorkerMsg) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", msg.to_line());
    let _ = out.flush();
}
