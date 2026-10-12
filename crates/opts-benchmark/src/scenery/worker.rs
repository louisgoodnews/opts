//! The worker process: a windowless Bevy app built on `opts-game`'s map loader.
//!
//! It loads the generated map once with `begin_world_load`, then for every object rewrites the
//! tile, rebuilds it through the loader (`plan_scene_reload` + `load_next_tile`, no file watcher),
//! waits until background mesh decoding has finished and the tile is stable, and captures the
//! views. The per-object analysis is in [`super::object`].

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
use opts_bevy::OmsiInstall;
use opts_game::in_game::omsi_world::{
    begin_world_load, load_next_tile, OmsiSessionConfig, WorldLoadJob, WorldStats,
};

use super::capture::{CaptureRig, ViewPixels};
use super::diag::{Baseline, EventSink};
use super::framing::View;
use super::mapgen::{MapWriter, Placement};
use super::protocol::WorkerMsg;
use super::{lookups, ObjectEntry};

/// Settings of one worker process.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Run directory (images, repro folders, the worker's map folder).
    pub run_dir: PathBuf,
    /// Worker number.
    pub id: usize,
    /// OMSI installation root.
    pub omsi_root: PathBuf,
    /// SQLite catalogue, opened read-only.
    pub catalogue: Option<PathBuf>,
    /// Relative path of the entry anchor `.sco`.
    pub anchor: String,
    /// Exit after this many objects (the supervisor starts a fresh worker).
    pub max_objects: usize,
    /// Time budget of one object.
    pub object_timeout: Duration,
    /// Which images to keep: `none`, `fail` (non-pass) or `all`.
    pub keep_images: String,
    /// Deliberate fault for the seeded-fault tests (`heading-mirror`, `offset-10m`, `scale-2x`).
    pub seed_fault: Option<String>,
    /// Decision thresholds.
    pub thresholds: super::verdict::Thresholds,
}

/// The running Bevy app plus the benchmark state around it.
pub struct Harness {
    apps: SubApps,
    rig: CaptureRig,
    /// The generated map.
    pub map: MapWriter,
    /// Installation used by preflight (same lookups as the loader).
    pub install: OmsiInstall,
    /// Captured `WARN`/`ERROR` events.
    pub sink: EventSink,
    /// Events of the empty map.
    pub baseline: Baseline,
    /// Settings.
    pub config: WorkerConfig,
}

impl Harness {
    /// Builds the app, writes the empty map and loads it.
    ///
    /// # Errors
    /// The map cannot be written or loaded, or it does not settle.
    pub fn new(config: WorkerConfig, sink: EventSink) -> Result<Self> {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .disable::<WinitPlugin>()
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::audio::AudioPlugin>()
                .disable::<bevy::gilrs::GilrsPlugin>(),
        );
        app.finish();
        app.cleanup();
        let mut apps = std::mem::take(app.sub_apps_mut());
        let rig = CaptureRig::spawn(apps.main.world_mut());

        let map = MapWriter {
            dir: config
                .run_dir
                .join(format!("work/worker-{}/map", config.id)),
            anchor: config.anchor.clone(),
        };
        map.write_initial()?;
        let catalogue = lookups::open(config.catalogue.as_deref());
        let install = lookups::install(&config.omsi_root, catalogue.as_ref());
        let session = OmsiSessionConfig {
            root: config.omsi_root.clone(),
            map_dir: map.dir.clone(),
            vehicle_path: None,
            entry_point: super::mapgen::ENTRY_NAME.into(),
            livery: String::new(),
            human_path: None,
            lookup: catalogue.as_ref().map(lookups::path_lookup),
            name_lookup: catalogue.as_ref().map(lookups::name_lookup),
            twin_lookup: catalogue.as_ref().map(lookups::twin_lookup),
            relocate_lookup: catalogue.as_ref().map(lookups::relocate_lookup),
        };
        let job = begin_world_load(session).map_err(|e| anyhow!("begin_world_load: {e}"))?;
        apps.main.world_mut().insert_resource(job);
        apps.main.world_mut().insert_resource(WorldStats::default());
        let mut h = Self {
            apps,
            rig,
            map,
            install,
            sink,
            baseline: Baseline::default(),
            config,
        };
        if !h.settle(Instant::now() + Duration::from_secs(60))? {
            return Err(anyhow!("the empty map did not settle"));
        }
        h.baseline = Baseline::from_events(&h.drain_events());
        Ok(h)
    }

    /// Runs one frame and waits for the GPU.
    pub fn update(&mut self) {
        self.apps.update();
        let world = self.apps.main.world();
        if let Some(device) = world.get_resource::<RenderDevice>() {
            let _ = device.wgpu_device().poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            });
        }
    }

    /// Takes the captured events.
    pub fn drain_events(&mut self) -> Vec<super::EventRecord> {
        self.sink
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }

    /// A copy of the loader counters.
    #[must_use]
    pub fn stats(&self) -> WorldStats {
        self.apps
            .main
            .world()
            .get_resource::<WorldStats>()
            .cloned()
            .unwrap_or_default()
    }

    /// Writes the tile (anchor plus `object`, if any) and makes the loader rebuild it from
    /// scratch (parsed objects and meshes are forgotten, so diagnostics repeat per object).
    ///
    /// # Errors
    /// The tile cannot be written.
    pub fn place(&mut self, object: Option<(&str, &[String], Placement)>) -> Result<()> {
        self.map.write_tile(object)?;
        let world = self.apps.main.world_mut();
        let plan = world
            .get_resource_mut::<WorldLoadJob>()
            .map(|mut job| job.plan_scene_reload())
            .ok_or_else(|| anyhow!("no world load job"))?;
        for e in plan.despawn {
            if let Ok(entity) = world.get_entity_mut(e) {
                entity.despawn();
            }
        }
        Ok(())
    }

    /// Loads queued tiles and waits for background decoding until the tile is stable for two
    /// consecutive checks. `Ok(false)` when `deadline` passes first.
    ///
    /// # Errors
    /// The loading system cannot run.
    pub fn settle(&mut self, deadline: Instant) -> Result<bool> {
        let mut stable = 0;
        while Instant::now() < deadline {
            self.apps
                .main
                .world_mut()
                .run_system_cached(drive_tiles)
                .map_err(|e| anyhow!("drive_tiles: {e}"))?;
            let world = self.apps.main.world();
            let done = world
                .get_resource::<WorldLoadJob>()
                .is_some_and(WorldLoadJob::tiles_done);
            let decoding = world
                .get_resource::<WorldStats>()
                .map_or(0, |s| s.meshes_decoding);
            if done && decoding == 0 {
                stable += 1;
                if stable >= 2 {
                    self.update();
                    return Ok(true);
                }
            } else {
                stable = 0;
                if decoding > 0 {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            self.update();
        }
        Ok(false)
    }

    /// Renders both views over black and white and reads them back.
    ///
    /// # Errors
    /// The readbacks do not arrive within a few frames.
    pub fn capture(&mut self, views: &[View; 2]) -> Result<[ViewPixels; 2]> {
        let world = self.apps.main.world_mut();
        self.rig.aim(world, views);
        let readers = self.rig.request(world);
        self.update();
        let world = self.apps.main.world_mut();
        self.rig.set_active(world, false);
        self.rig.stop_reading(world, &readers);
        let mut out = None;
        for _ in 0..5 {
            self.update();
            out = self.rig.take();
            if out.is_some() {
                break;
            }
        }
        self.rig.despawn(self.apps.main.world_mut(), readers);
        out.ok_or_else(|| anyhow!("readback did not arrive"))
    }
}

/// Rebuilds tiles whose meshes finished decoding and loads every queued tile.
fn drive_tiles(
    mut job: ResMut<WorldLoadJob>,
    mut stats: ResMut<WorldStats>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    job.rebuild_decoded_tiles(&mut commands, &mut stats);
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
