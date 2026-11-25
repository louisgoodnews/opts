//! Where a requested vehicle is placed: at a bus stop, at an entry point, or at the given place.

use super::*;

/// A vehicle that starts at a bus stop: placed like a trip's start, and only when the stop is free.
pub(super) fn place_at_stop(
    (job, commands, registry): (&mut WorldLoadJob, &mut Commands, &mut SessionVehicles),
    vehicles: &Query<&GlobalTransform, VehicleFilter>,
    (id, rec, bus_path, formation_cars): (
        opts_game_state::session_vehicles::VehicleId,
        &opts_game_state::session_vehicles::SessionVehicleRecord,
        PathBuf,
        Vec<super::super::FormationCar>,
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
pub(super) fn place_at_entry(
    (job, commands, registry): (&mut WorldLoadJob, &mut Commands, &mut SessionVehicles),
    entries: &EntryIndex,
    (id, rec, bus_path, formation_cars): (
        opts_game_state::session_vehicles::VehicleId,
        &opts_game_state::session_vehicles::SessionVehicleRecord,
        PathBuf,
        Vec<super::super::FormationCar>,
    ),
) {
    let entry = match &rec.entry {
        opts_game_state::session_vehicles::EntrySel::Id(i) => job.entry_name(*i).map(|n| (*i, n)),
        opts_game_state::session_vehicles::EntrySel::Name(n) => job.entry_by_name(n),
        opts_game_state::session_vehicles::EntrySel::Default | opts_game_state::session_vehicles::EntrySel::Stop(_) => {
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

/// How far from a bus stop another vehicle must stand for the stop to count as free (m): about a
/// bus length from the place the new vehicle takes.
pub(super) const STOP_CLEAR_M: f32 = 10.0;

/// Places the vehicle of request `id` at `local` of `tile` and asks for it to be built.
pub(super) fn place(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (id, rec, bus_path, formation_cars): (
        opts_game_state::session_vehicles::VehicleId,
        &opts_game_state::session_vehicles::SessionVehicleRecord,
        PathBuf,
        Vec<super::super::FormationCar>,
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
