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

mod entry_index;
pub use entry_index::{EntryIndex, ResolvedEntry};

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

/// Traffic and session vehicles (what makes a bus stop occupied).
type VehicleFilter = Or<(
    With<super::traffic::TrafficVehicle>,
    With<opts_bevy::VehicleController>,
)>;

/// Finishes the spawn that was started in an earlier frame: builds the vehicle and makes it the
/// active one, or marks the request failed and says so in a toast.
fn complete_flight(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    assets: &mut LoadAssets,
    (registry, camera): (&mut SessionVehicles, &mut CameraState),
    (loc, flight): (Option<&Localization>, &SpawnVehicleRequest),
) {
    let id = flight.id;
    match load_player_vehicle(
        (job, stats, commands),
        (
            &mut assets.meshes,
            &mut assets.materials,
            &mut assets.images,
            &mut assets.bindposes,
        ),
        (id, false),
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
                .spawn(commands);
            }
        }
    }
}

/// A vehicle that starts at a bus stop: placed like a trip's start, and only when the stop is free.
fn place_at_stop(
    (job, commands, registry): (&mut WorldLoadJob, &mut Commands, &mut SessionVehicles),
    vehicles: &Query<&GlobalTransform, VehicleFilter>,
    (id, rec, bus_path, formation_cars): (
        crate::session_vehicles::VehicleId,
        &crate::session_vehicles::SessionVehicleRecord,
        PathBuf,
        Vec<super::FormationCar>,
    ),
    station: u32,
) {
    job.config.vehicle_path = Some(bus_path.clone());
    let keys: Vec<(i32, i32)> = job.tile_files.keys().copied().collect();
    let Some((tile, (local, heading))) = job.find_stop_spawn(station, &keys) else {
        registry.mark_failed(
            id,
            format!("no bus stop with the station id {station} on this map"),
        );
        return;
    };
    let session = job.session_pos(tile, local);
    let at = Vec2::new(session.x, -session.y);
    if let Some(d) = vehicles
        .iter()
        .map(|g| g.translation().xz().distance(at))
        .find(|d| *d < STOP_CLEAR_M)
    {
        registry.mark_failed(
            id,
            format!("the stop is occupied (a vehicle {d:.1} m from it)"),
        );
        return;
    }
    let name = job
        .timetables()
        .and_then(|t| t.stop(station).map(|s| s.name.clone()))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("stop {station}"));
    place(
        job,
        commands,
        (id, rec, bus_path, formation_cars),
        (tile, local, heading),
        (0, name),
    );
}

/// A vehicle that starts at an entry point: the resolved entry, or (entries on spline attachments)
/// a search of the map.
fn place_at_entry(
    (job, commands, registry): (&mut WorldLoadJob, &mut Commands, &mut SessionVehicles),
    entries: &EntryIndex,
    (id, rec, bus_path, formation_cars): (
        crate::session_vehicles::VehicleId,
        &crate::session_vehicles::SessionVehicleRecord,
        PathBuf,
        Vec<super::FormationCar>,
    ),
) {
    let entry = match &rec.entry {
        crate::session_vehicles::EntrySel::Id(i) => job.entry_name(*i).map(|n| (*i, n)),
        crate::session_vehicles::EntrySel::Name(n) => job.entry_by_name(n),
        crate::session_vehicles::EntrySel::Default | crate::session_vehicles::EntrySel::Stop(_) => {
            job.configured_entry()
        }
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
    place(
        job,
        commands,
        (id, rec, bus_path, formation_cars),
        (tile, local, heading),
        (entry_id, entry_name),
    );
}

/// What the spawn-request system reads and writes besides the world-load state.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RequestContext<'w, 's> {
    registry: ResMut<'w, SessionVehicles>,
    flight: Option<Res<'w, SpawnVehicleRequest>>,
    entries: Res<'w, EntryIndex>,
    camera: ResMut<'w, CameraState>,
    loc: Option<Res<'w, Localization>>,
    vehicles: Query<'w, 's, &'static GlobalTransform, VehicleFilter>,
}

/// Works down the spawn requests of the registry: places the vehicle at its entry point, builds it
/// and makes it the active vehicle.
pub fn handle_vehicle_requests(
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    mut stats: ResMut<WorldStats>,
    mut assets: LoadAssets,
    ctx: RequestContext,
) {
    let RequestContext {
        mut registry,
        flight,
        entries,
        mut camera,
        loc,
        vehicles,
    } = ctx;
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
        complete_flight(
            (&mut job, &mut stats, &mut commands),
            &mut assets,
            (&mut registry, &mut camera),
            (loc.as_deref(), &flight),
        );
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
                let text = join_issues(&issues);
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
    if let crate::session_vehicles::EntrySel::Stop(station) = rec.entry {
        place_at_stop(
            (&mut job, &mut commands, &mut registry),
            &vehicles,
            (id, &rec, bus_path, formation_cars),
            station,
        );
        return;
    }
    place_at_entry(
        (&mut job, &mut commands, &mut registry),
        &entries,
        (id, &rec, bus_path, formation_cars),
    );
}

/// The issues of a formation as one text.
fn join_issues<T: ToString>(issues: &[T]) -> String {
    issues
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// How far from a bus stop another vehicle must stand for the stop to count as free (m): about a
/// bus length from the place the new vehicle takes.
const STOP_CLEAR_M: f32 = 10.0;

/// Places the vehicle of request `id` at `local` of `tile` and asks for it to be built.
fn place(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (id, rec, bus_path, formation_cars): (
        crate::session_vehicles::VehicleId,
        &crate::session_vehicles::SessionVehicleRecord,
        PathBuf,
        Vec<super::FormationCar>,
    ),
    (tile, local, heading): ((i32, i32), Vec3, f32),
    entry: (u32, String),
) {
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
        entry,
        point: SpawnPoint {
            x: abs.x,
            y: abs.y,
            z: f64::from(local.z),
            heading_deg: f64::from(heading),
        },
    });
}
