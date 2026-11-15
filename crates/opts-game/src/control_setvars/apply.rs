//! Applies one changed option to the driven vehicle: its scripts, its session record and its load job.

use bevy::prelude::*;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::session_vehicles::SessionVehicles;

/// A change of the driven vehicle: set on its scripts at once, kept on its session record (the
/// save, a saved configuration and the editor read it there) and, while the driven vehicle is
/// the one the drive started with, on the load job too.
pub fn apply_live(world: &mut World, name: &str, value: i32, default: i32) {
    let active = world.get_resource::<SessionVehicles>().and_then(|r| {
        r.active()
            .filter(|r| r.is_alive())
            .map(|r| (r.id, r.bus_file.clone()))
    });
    if let Some((id, _)) = &active {
        world
            .resource_mut::<SessionVehicles>()
            .set_setvar(*id, name, value, default);
    }
    if let Some(mut job) = world.get_resource_mut::<WorldLoadJob>() {
        let first = match &active {
            Some((_, bus)) => job
                .vehicle_file()
                .is_some_and(|f| f == std::path::Path::new(bus)),
            None => true,
        };
        if first {
            job.set_vehicle_setvar(name, value);
        }
    }
    let mut q =
        world.query_filtered::<&opts_bevy::VehicleScripts, With<opts_bevy::PlayerVehicle>>();
    if let Some(scripts) = q.iter(world).next() {
        if let Ok(mut engine) = scripts.engine.lock() {
            engine.set(name, value as f32);
        }
    }
}
