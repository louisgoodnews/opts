//! Spawning session vehicles while a session runs (the in-game vehicle menu, the control protocol):
//! the entry points of the map resolved to world positions, and the spawner that works down the
//! spawn requests of the [`SessionVehicles`] registry and builds each vehicle at its entry point.
//! Despawning, switching and respawning are in `crate::session_vehicles::ops`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use opts_bevy::{CameraMode, CameraState};
use opts_core::formats::map::{MapEntryPoint, MapTile};

use super::vehicle::load_player_vehicle;
use super::{LoadAssets, WorldLoadJob, WorldStats};
use crate::session_vehicles::{ops, SessionVehicles, SpawnPoint, VehicleId};
use opts_gui::l10n::Localization;
use opts_gui::prelude::ToastBuilder;

/// An `[entrypoints]` record resolved to the position of its placed `entrypoint_*.sco`.
#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    /// Record number of the `[entrypoints]` entry (several records can share a name).
    pub id: u32,
    /// Display name (`global.cfg`).
    pub name: String,
    /// Tile the entry object lies on.
    pub tile: (i32, i32),
    /// Position in that tile's frame, with the record's elevation as `z`.
    pub local: Vec3,
    /// Heading of the entry object (compass degrees).
    pub heading_deg: f32,
}

/// What the worker has found so far, and whether it has finished.
#[derive(Default)]
struct Found {
    entries: Vec<ResolvedEntry>,
    done: bool,
}

type SharedEntries = Arc<Mutex<Found>>;

/// The map's entry points with their positions, built once per session on a worker thread (every
/// tile is read to find the placed entry objects, nearest tiles first). Results are published as
/// they are found, so the entries near the player can be checked long before the last tile is
/// read.
#[derive(Resource, Default)]
pub struct EntryIndex {
    shared: Option<SharedEntries>,
    found: Vec<ResolvedEntry>,
    done: bool,
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
        self.found.clear();
        self.done = false;
        let start = job.start_tile;
        let mut tiles: Vec<((i32, i32), PathBuf)> = job
            .tile_files
            .keys()
            .filter_map(|k| job.tile_path(*k).map(|p| (*k, p)))
            .collect();
        tiles.sort_by_key(|((x, y), _)| (x - start.0).abs().max((y - start.1).abs()));
        let entries = job.manifest.entry_points.clone();
        let shared: SharedEntries = Arc::new(Mutex::new(Found::default()));
        self.shared = Some(shared.clone());
        std::thread::spawn(move || resolve_entries(&tiles, &entries, &shared));
    }

    /// Takes what the worker has found so far.
    pub fn poll(&mut self) {
        if self.done {
            return;
        }
        if let Some(shared) = &self.shared {
            if let Ok(f) = shared.lock() {
                if f.entries.len() != self.found.len() || f.done != self.done {
                    self.found = f.entries.clone();
                    self.done = f.done;
                }
            }
        }
    }

    /// The entries resolved so far.
    #[must_use]
    pub fn entries(&self) -> &[ResolvedEntry] {
        &self.found
    }

    /// Whether every tile has been read.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// The resolved entry with that record number.
    #[must_use]
    pub fn find(&self, id: u32) -> Option<&ResolvedEntry> {
        self.found.iter().find(|e| e.id == id)
    }
}

/// Reads the tiles and finds the placed object of each entry record (the same rule as
/// `WorldLoadJob::find_entry_spawn`; entries on spline attachments are left out).
fn resolve_entries(
    tiles: &[((i32, i32), PathBuf)],
    entries: &[MapEntryPoint],
    shared: &SharedEntries,
) {
    let mut found = 0usize;
    for (key, path) in tiles {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(tile) = MapTile::parse(&bytes) else {
            continue;
        };
        let mut fresh = Vec::new();
        for ep in entries {
            if let Some(obj) = tile
                .objects
                .iter()
                .find(|o| o.id_code == u64::from(ep.id_code))
            {
                fresh.push(ResolvedEntry {
                    id: ep.id,
                    name: ep.name.clone(),
                    tile: *key,
                    local: Vec3::new(obj.position.x, obj.position.y, ep.position.y),
                    heading_deg: obj.heading_deg,
                });
            }
        }
        if !fresh.is_empty() {
            if let Ok(mut f) = shared.lock() {
                for r in fresh {
                    if !f.entries.iter().any(|e| e.id == r.id) {
                        f.entries.push(r);
                        found += 1;
                    }
                }
            }
        }
        if found >= entries.len() {
            break;
        }
    }
    if let Ok(mut f) = shared.lock() {
        f.done = true;
    }
}

/// The spawn request being worked on: it is built on the next frame. One at a time; the others
/// wait in the registry as [`Lifecycle::Spawning`] records.
///
/// [`Lifecycle::Spawning`]: crate::session_vehicles::Lifecycle::Spawning
#[derive(Resource)]
pub struct SpawnVehicleRequest {
    /// The registry id of the vehicle being built.
    pub id: VehicleId,
    entry: (u32, String),
    point: SpawnPoint,
}

/// Works down the spawn requests of the registry: places the vehicle at its entry point, builds it
/// and makes it the active vehicle.
#[allow(clippy::too_many_arguments)]
pub fn handle_vehicle_requests(
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    mut stats: ResMut<WorldStats>,
    mut assets: LoadAssets,
    mut registry: ResMut<SessionVehicles>,
    flight: Option<Res<SpawnVehicleRequest>>,
    entries: Res<EntryIndex>,
    mut camera: ResMut<CameraState>,
    loc: Option<Res<Localization>>,
) {
    let Some(mut job) = job else {
        return;
    };
    let in_flight = flight.as_ref().map(|f| f.id);
    if let Some(flight) = flight {
        // Cancelled (despawned before it existed): drop the request.
        let wanted = registry
            .get(flight.id)
            .is_some_and(|r| r.lifecycle == crate::session_vehicles::Lifecycle::Spawning);
        if !wanted {
            commands.remove_resource::<SpawnVehicleRequest>();
            return;
        }
        let id = flight.id;
        match load_player_vehicle(
            &mut job,
            &mut stats,
            &mut commands,
            &mut assets.meshes,
            &mut assets.materials,
            &mut assets.images,
            &mut assets.bindposes,
            id,
            false,
        ) {
            Ok(spawned) => {
                registry.mark_alive(
                    id,
                    spawned.root,
                    Some(flight.entry.clone()),
                    Some(flight.point),
                    Some(spawned.camera),
                );
                camera.mode = CameraMode::External;
                tracing::info!(id, entry = %flight.entry.1, "session vehicle spawned");
                commands.queue(move |world: &mut World| {
                    if let Err(e) = ops::switch_to(world, id) {
                        tracing::error!(at = %opts_bevy::position_tag(), "cannot activate the new vehicle: {e}");
                    }
                });
            }
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), "cannot spawn the vehicle: {e}");
                let name = registry.get(id).map(|r| r.name.clone()).unwrap_or_default();
                registry.mark_failed(id, e.clone());
                if let Some(loc) = loc {
                    ToastBuilder::new(loc.t_args(
                        crate::shell::keys::SPAWN_FAILED,
                        &[
                            ("name".to_owned(), name.as_str().into()),
                            ("error".to_owned(), e.as_str().into()),
                        ],
                    ))
                    .with_duration(8.0)
                    .spawn(&mut commands);
                }
            }
        }
        commands.remove_resource::<SpawnVehicleRequest>();
        return;
    }
    let Some(id) = registry.next_pending(in_flight) else {
        return;
    };
    let Some(rec) = registry.get(id).cloned() else {
        return;
    };
    let mut bus_path = PathBuf::from(&rec.bus_file);
    // A train formation: its first car is the vehicle, the others are coupled behind it by the loader.
    let formation_cars = match &rec.formation {
        Some(formation) => match crate::formation::resolve(formation, &job) {
            Ok(resolved) => {
                bus_path = resolved.head;
                resolved.cars
            }
            Err(issues) => {
                let text = issues
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                tracing::error!(at = %opts_bevy::position_tag(), formation = %formation.name, "formation cannot be spawned: {text}");
                registry.mark_failed(id, text);
                return;
            }
        },
        None => Vec::new(),
    };
    if !bus_path.is_file() {
        registry.mark_failed(id, format!("{}: no such vehicle file", rec.bus_file));
        tracing::error!(at = %opts_bevy::position_tag(), file = %rec.bus_file, "vehicle file not found; nothing spawned");
        return;
    }
    // Where: the resolved entry, or (entries on spline attachments) a search of the map.
    let entry = match &rec.entry {
        crate::session_vehicles::EntrySel::Id(i) => job.entry_name(*i).map(|n| (*i, n)),
        crate::session_vehicles::EntrySel::Name(n) => job.entry_by_name(n),
        crate::session_vehicles::EntrySel::Default => job.configured_entry(),
    };
    let Some((entry_id, entry_name)) = entry else {
        registry.mark_failed(id, "no such entry point on this map".to_owned());
        tracing::error!(at = %opts_bevy::position_tag(), entry = ?rec.entry, "entry point not found; no vehicle spawned");
        return;
    };
    let placed = entries
        .find(entry_id)
        .map(|e| (e.tile, e.local, e.heading_deg))
        .or_else(|| {
            let keys: Vec<(i32, i32)> = job.tile_files.keys().copied().collect();
            job.find_entry_spawn_by_id(entry_id, &keys)
                .map(|(k, (p, h))| (k, p, h))
        });
    let Some((tile, local, heading)) = placed else {
        registry.mark_failed(id, format!("entry point `{entry_name}` cannot be placed"));
        tracing::error!(at = %opts_bevy::position_tag(), entry = %entry_id, "entry point cannot be placed; no vehicle spawned");
        return;
    };
    let session = job.session_pos(tile, local);
    let abs = job.grid.absolute(tile, local.truncate().as_dvec2());
    job.spawn = Some((session, heading));
    job.config.vehicle_path = Some(bus_path);
    job.set_formation_cars(formation_cars);
    job.config.livery = rec.repaint.clone();
    job.config.setvars = rec.setvars.clone();
    job.repaint = None;
    commands.insert_resource(SpawnVehicleRequest {
        id,
        entry: (entry_id, entry_name),
        point: SpawnPoint {
            x: abs.x,
            y: abs.y,
            z: f64::from(local.z),
            heading_deg: f64::from(heading),
        },
    });
}
