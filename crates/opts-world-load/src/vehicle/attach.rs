//! Attaching a vehicle's scripts, sounds, passengers and key bindings, and its trailing units.

use super::*;

/// The scripts, sounds, door points, passengers and key bindings of the vehicle.
pub(super) fn attach_vehicle_parts(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    (manifest, bus_dir): (&BusManifest, &Path),
    (script_seed, activate): (u64, bool),
) {
    attach_scripts(job, commands, root, manifest, bus_dir, script_seed, None);
    job.sound()
        .attach_player(job, commands, root, manifest, bus_dir);
    doors::attach_door_points(job, commands, root, manifest, bus_dir);
    if activate {
        passengers::attach_passengers(job, commands, root, manifest, bus_dir, script_seed);
    }
    load_key_bindings(job, commands);
}

/// The trailing units of an articulated vehicle and the cameras they bring.
pub(super) fn attach_trailers(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    root: Entity,
    (bus_dir, manifest): (&Path, &BusManifest),
) {
    let (trailer_cams, trailer_driver_cams) = spawn_trailer_chain(
        job,
        commands,
        meshes,
        materials,
        images,
        root,
        (bus_dir, manifest),
    );
    if !trailer_cams.is_empty() || !trailer_driver_cams.is_empty() {
        commands.queue(move |world: &mut World| {
            if let Some(mut bus) = world.get_mut::<opts_bevy::BusVehicle>(root) {
                bus.trailer_pax_cameras = trailer_cams;
                bus.trailer_driver_cameras = trailer_driver_cams;
            }
        });
    }
}
