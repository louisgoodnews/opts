//! The vehicle requests of a running session: the request, its context and the handler.

use super::*;

/// The spawn request being worked on: it is built on the next frame. One at a time; the others
/// wait in the registry as [`Lifecycle::Spawning`] records.
///
/// [`Lifecycle::Spawning`]: opts_game_state::session_vehicles::Lifecycle::Spawning
#[derive(Resource)]
pub struct SpawnVehicleRequest {
    /// The registry id of the vehicle being built.
    pub id: VehicleId,
    pub(super) entry: (u32, String),
    pub(super) point: SpawnPoint,
}

/// Traffic and session vehicles (what makes a bus stop occupied).
pub(super) type VehicleFilter = Or<(
    With<super::super::traffic::TrafficVehicle>,
    With<opts_bevy::VehicleController>,
)>;

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
            .is_some_and(|r| r.lifecycle == opts_game_state::session_vehicles::Lifecycle::Spawning);
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
    if let opts_game_state::session_vehicles::EntrySel::Stop(station) = rec.entry {
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
