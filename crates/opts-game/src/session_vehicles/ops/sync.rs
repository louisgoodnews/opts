//! Noticing vanished session vehicles and refreshing the records from the live chains.

use bevy::prelude::*;
use opts_bevy::{ArticulatedChain, VehicleController};
use opts_gui::l10n::Localization;
use opts_gui::prelude::ToastBuilder;

use crate::session_vehicles::registry::{
    DespawnReason, Lifecycle, SessionVehicles, SpawnPoint, VehicleId,
};

use super::despawn::despawn;
use super::frame::{local_to_abs, PLACE_LABEL_HIDE_DISTANCE_M};

/// Notices vehicles whose entity vanished without a request (destroyed, or removed with a tile):
/// they are marked [`DespawnReason::Lost`], the next vehicle becomes active and a warning toast
/// names the vehicle. No entity of a lost vehicle is left dangling (its trailer parts go too).
pub fn sync_lost(world: &mut World) {
    /// Frames an entity may be missing before it counts as gone: a vehicle that was just built
    /// exists only once the spawner's commands have been applied.
    const GRACE_FRAMES: u32 = 3;
    let alive: Vec<(VehicleId, bool)> = {
        let reg = world.resource::<SessionVehicles>();
        reg.records()
            .iter()
            .filter(|r| r.lifecycle == Lifecycle::Alive)
            .map(|r| (r.id, r.entity.is_some_and(|e| world.get_entity(e).is_ok())))
            .collect()
    };
    let mut lost = Vec::new();
    for (id, present) in alive {
        let missing = world
            .resource_mut::<SessionVehicles>()
            .get_mut(id)
            .map(|r| {
                r.missing_frames = if present { 0 } else { r.missing_frames + 1 };
                r.missing_frames
            });
        if missing.is_some_and(|m| m > GRACE_FRAMES) {
            lost.push(id);
        }
    }
    for id in lost {
        let (name, trailers) = {
            let reg = world.resource::<SessionVehicles>();
            reg.get(id)
                .map(|r| (r.name.clone(), r.trailers.clone()))
                .unwrap_or_default()
        };
        for t in trailers {
            if world.get_entity(t).is_ok() {
                world.entity_mut(t).despawn();
            }
        }
        let _ = despawn(world, id, DespawnReason::Lost);
        tracing::warn!(id, vehicle = %name, "a session vehicle vanished from the world");
        let text = world.get_resource::<Localization>().map_or_else(
            || name.clone(),
            |l| {
                l.t_args(
                    crate::shell::keys::VEHICLE_LOST,
                    &[("name".to_owned(), name.as_str().into())],
                )
            },
        );
        let mut commands = world.commands();
        ToastBuilder::new(text)
            .with_duration(6.0)
            .spawn(&mut commands);
        world.flush();
    }
}

/// Refreshes the trailer lists of the records from the live chains, and the latch of the place
/// label: a vehicle that has been further than [`PLACE_LABEL_HIDE_DISTANCE_M`] from its spawn
/// point no longer shows it.
pub fn refresh_records(world: &mut World) {
    let ids: Vec<(VehicleId, Entity, Option<SpawnPoint>)> = world
        .resource::<SessionVehicles>()
        .records()
        .iter()
        .filter(|r| r.lifecycle == Lifecycle::Alive)
        .filter_map(|r| Some((r.id, r.entity?, r.spawn_point)))
        .collect();
    for (id, entity, point) in ids {
        let trailers: Vec<Entity> = world
            .get::<ArticulatedChain>(entity)
            .map(|c| c.segments.clone())
            .unwrap_or_default();
        let away = point.and_then(|p| {
            let c = world.get::<VehicleController>(entity)?;
            let (x, y) = local_to_abs(world, c.sim.chassis.position);
            let (dx, dy) = (x - p.x, y - p.y);
            Some(((dx * dx + dy * dy).sqrt() as f32) > PLACE_LABEL_HIDE_DISTANCE_M)
        });
        if let Some(rec) = world.resource_mut::<SessionVehicles>().get_mut(id) {
            if rec.trailers != trailers {
                rec.trailers = trailers;
            }
            if away == Some(true) && !rec.place_dismissed {
                rec.place_dismissed = true;
            }
        }
    }
}
