//! Completing a vehicle's flight: the flight's end, and the joined list of problems.

use super::*;

/// Finishes the spawn that was started in an earlier frame: builds the vehicle and makes it the
/// active one, or marks the request failed and says so in a toast.
pub(super) fn complete_flight(
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

/// The issues of a formation as one text.
pub(super) fn join_issues<T: ToString>(issues: &[T]) -> String {
    issues
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
