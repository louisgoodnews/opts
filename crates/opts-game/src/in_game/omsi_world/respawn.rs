//! Despawning and spawning the player's vehicle while a session runs (the in-game vehicle menu,
//! `in_game/vehicle_menu.rs`): the entry points of the map resolved to world positions, and the
//! requests that remove the current vehicle and build a new one at an entry point.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use opts_bevy::{ArticulatedChain, CameraMode, CameraState, PlayerVehicle};
use opts_core::formats::map::{MapEntryPoint, MapTile};

use super::ground::tile_origin;
use super::rail::VehicleComposition;
use super::vehicle::{decodes_pending, load_player_vehicle, prewarm_vehicle_meshes};
use super::{LoadAssets, WorldLoadJob, WorldStats};

/// An `[entrypoints]` record resolved to the position of its placed `entrypoint_*.sco`.
#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    /// Record number of the `[entrypoints]` entry (several records can share a name).
    pub id: u32,
    /// Display name (`global.cfg`).
    pub name: String,
    /// Absolute OMSI world position: tile position plus the record's elevation.
    pub world: Vec3,
    /// Heading of the entry object (compass degrees).
    pub heading_deg: f32,
}

type SharedEntries = Arc<Mutex<Option<Vec<ResolvedEntry>>>>;

/// The map's entry points with their positions, built once per session on a worker thread (every
/// tile is read to find the placed entry objects).
#[derive(Resource, Default)]
pub struct EntryIndex {
    shared: Option<SharedEntries>,
    ready: Option<Vec<ResolvedEntry>>,
    map: Option<PathBuf>,
}

impl EntryIndex {
    /// Forgets everything (a new session starts).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Starts resolving the entry points of the loaded map (idempotent).
    pub fn start(&mut self, job: &WorldLoadJob) {
        if self.map.as_ref() == Some(&job.config.map_dir) {
            return;
        }
        self.map = Some(job.config.map_dir.clone());
        self.ready = None;
        let tiles: Vec<((i32, i32), PathBuf)> = job
            .tile_files
            .keys()
            .filter_map(|k| job.tile_path(*k).map(|p| (*k, p)))
            .collect();
        let entries = job.manifest.entry_points.clone();
        let shared: SharedEntries = Arc::new(Mutex::new(None));
        self.shared = Some(shared.clone());
        std::thread::spawn(move || {
            let out = resolve_entries(&tiles, &entries);
            if let Ok(mut s) = shared.lock() {
                *s = Some(out);
            }
        });
    }

    /// Takes the worker's result once it is there.
    pub fn poll(&mut self) {
        if self.ready.is_some() {
            return;
        }
        if let Some(shared) = &self.shared {
            if let Ok(mut s) = shared.lock() {
                self.ready = s.take();
            }
        }
    }

    /// The resolved entries, once the worker has finished.
    #[must_use]
    pub fn entries(&self) -> Option<&[ResolvedEntry]> {
        self.ready.as_deref()
    }

    /// The resolved entry with that record number.
    #[must_use]
    pub fn find(&self, id: u32) -> Option<&ResolvedEntry> {
        self.ready.as_ref()?.iter().find(|e| e.id == id)
    }
}

/// Reads every tile and finds the placed object of each entry record (the same rule as
/// `WorldLoadJob::find_entry_spawn`; entries on spline attachments are left out).
fn resolve_entries(
    tiles: &[((i32, i32), PathBuf)],
    entries: &[MapEntryPoint],
) -> Vec<ResolvedEntry> {
    let mut out: Vec<ResolvedEntry> = Vec::new();
    for (key, path) in tiles {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(tile) = MapTile::parse(&bytes) else {
            continue;
        };
        let origin = tile_origin(*key);
        for ep in entries {
            if out.iter().any(|r| r.id == ep.id) {
                continue;
            }
            if let Some(obj) = tile
                .objects
                .iter()
                .find(|o| o.id_code == u64::from(ep.id_code))
            {
                out.push(ResolvedEntry {
                    id: ep.id,
                    name: ep.name.clone(),
                    world: Vec3::new(
                        origin.0 + obj.position.x,
                        origin.1 + obj.position.y,
                        ep.position.y,
                    ),
                    heading_deg: obj.heading_deg,
                });
            }
        }
        if out.len() == entries.len() {
            break;
        }
    }
    out
}

/// Request to remove the player's vehicle.
#[derive(Resource)]
pub struct DespawnVehicleRequest;

/// Request to build a vehicle at an entry point (replacing the current one).
#[derive(Resource)]
pub struct SpawnVehicleRequest {
    /// The `.bus` file.
    pub bus_path: PathBuf,
    /// Chosen repaint (empty: the vehicle's default).
    pub livery: String,
    /// Equipment options set on the vehicle's scripts.
    pub setvars: Vec<(String, i32)>,
    /// Record number of the entry point.
    pub entry: u32,
    stage: SpawnStage,
}

impl SpawnVehicleRequest {
    /// A request for `bus_path` at the entry named `entry`.
    #[must_use]
    pub fn new(bus_path: PathBuf, livery: String, setvars: Vec<(String, i32)>, entry: u32) -> Self {
        Self {
            bus_path,
            livery,
            setvars,
            entry,
            stage: SpawnStage::Remove,
        }
    }
}

enum SpawnStage {
    /// Waiting for the old vehicle to be gone.
    Remove,
    /// The new vehicle's scrambled meshes decode in the background.
    Decode(Vec<opts_bevy::descramble_cache::Key>),
}

/// Despawns a player vehicle with all its parts.
fn despawn_vehicle(
    commands: &mut Commands,
    root: Entity,
    chain: Option<&ArticulatedChain>,
    composition: Option<&VehicleComposition>,
) {
    // The composition lists the chain's segments too: despawn each entity once.
    let mut parts: std::collections::HashSet<Entity> = chain
        .into_iter()
        .flat_map(|c| c.segments.iter().copied())
        .collect();
    parts.extend(
        composition
            .into_iter()
            .flat_map(VehicleComposition::parts)
            .map(|p| p.entity),
    );
    parts.remove(&root);
    for e in parts {
        commands.entity(e).despawn();
    }
    commands.entity(root).despawn();
}

/// The root and the part lists of a player vehicle.
type PlayerParts = (
    Entity,
    Option<&'static ArticulatedChain>,
    Option<&'static VehicleComposition>,
);

/// Handles [`DespawnVehicleRequest`] and [`SpawnVehicleRequest`].
#[allow(clippy::too_many_arguments)]
pub fn handle_vehicle_requests(
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    mut stats: ResMut<WorldStats>,
    mut assets: LoadAssets,
    despawn: Option<Res<DespawnVehicleRequest>>,
    spawn: Option<ResMut<SpawnVehicleRequest>>,
    entries: Res<EntryIndex>,
    mut camera: ResMut<CameraState>,
    players: Query<PlayerParts, With<PlayerVehicle>>,
) {
    if despawn.is_some() {
        commands.remove_resource::<DespawnVehicleRequest>();
        for (root, chain, comp) in &players {
            despawn_vehicle(&mut commands, root, chain, comp);
        }
        camera.mode = CameraMode::FreeRoam;
        tracing::info!("player vehicle despawned");
    }
    let (Some(mut spawn), Some(mut job)) = (spawn, job) else {
        return;
    };
    match &mut spawn.stage {
        SpawnStage::Remove => {
            if despawn.is_none() && !players.is_empty() {
                for (root, chain, comp) in &players {
                    despawn_vehicle(&mut commands, root, chain, comp);
                }
                return; // the entities are gone from the next frame on
            }
            if !players.is_empty() {
                return;
            }
            // Where: the resolved entry, or (entries on spline attachments) a search of the map.
            let placed = entries
                .find(spawn.entry)
                .map(|e| (e.world, e.heading_deg))
                .or_else(|| {
                    let keys: Vec<(i32, i32)> = job.tile_files.keys().copied().collect();
                    job.find_entry_spawn_by_id(spawn.entry, &keys)
                        .map(|(key, (p, h))| {
                            let _ = key;
                            (p, h)
                        })
                });
            let Some((world, heading)) = placed else {
                tracing::error!(at = %opts_bevy::position_tag(), entry = %spawn.entry, "entry point cannot be placed; no vehicle spawned");
                commands.remove_resource::<SpawnVehicleRequest>();
                return;
            };
            let abs = tile_origin(job.start_tile);
            job.spawn = Some((
                Vec3::new(world.x - abs.0, world.y - abs.1, world.z),
                heading,
            ));
            job.config.vehicle_path = Some(spawn.bus_path.clone());
            job.config.livery = spawn.livery.clone();
            job.config.setvars = spawn.setvars.clone();
            job.repaint = None;
            let keys = prewarm_vehicle_meshes(
                &mut job,
                &mut commands,
                &mut assets.meshes,
                &mut assets.materials,
                &mut assets.images,
            );
            spawn.stage = SpawnStage::Decode(keys);
        }
        SpawnStage::Decode(keys) => {
            if decodes_pending(&job, keys) > 0 {
                return;
            }
            match load_player_vehicle(
                &mut job,
                &mut stats,
                &mut commands,
                &mut assets.meshes,
                &mut assets.materials,
                &mut assets.images,
                &mut assets.bindposes,
            ) {
                Ok(()) => {
                    camera.mode = CameraMode::External;
                    tracing::info!(entry = %spawn.entry, "player vehicle spawned");
                }
                Err(e) => {
                    tracing::error!(at = %opts_bevy::position_tag(), "cannot spawn the vehicle: {e}");
                }
            }
            commands.remove_resource::<SpawnVehicleRequest>();
        }
    }
}
