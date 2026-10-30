//! The steps of building the player's vehicle around its spawned root.

use super::*;

/// Where the vehicle stands: the start road spline's lane position in Bevy space, as the world
/// position and heading it came from plus its root transform.
pub(super) fn spawn_pose(job: &WorldLoadJob) -> Result<(Vec3, f32, Transform), String> {
    let (spawn_world, heading_deg) = job.spawn.ok_or("no spawn point")?;
    let bevy_pos = omsi_to_bevy_pos(spawn_world);
    let root_tf = Transform {
        translation: bevy_pos,
        rotation: omsi_euler_to_bevy_quat(-heading_deg, 0.0, 0.0),
        scale: Vec3::ONE,
    };
    Ok((spawn_world, heading_deg, root_tf))
}

/// The directories whose files the hot reload watches for the vehicle.
pub(super) fn set_vehicle_dirs(
    job: &mut WorldLoadJob,
    (bus_dir, model_dir, tex_dirs): (&Path, &Path, &[PathBuf]),
) {
    job.vehicle_dirs = std::iter::once(bus_dir.to_path_buf())
        .chain(std::iter::once(model_dir.to_path_buf()))
        .chain(tex_dirs.iter().cloned())
        .map(|d| opts_bevy::hot_reload::cache::normalize(&d))
        .collect();
}

/// Components, scripts, trailers and driver of the vehicle; returns the driver's eye point.
pub(super) fn assemble_vehicle(
    (job, commands): (&mut WorldLoadJob, &mut Commands),
    (meshes, materials, images, bindposes): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
        &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    ),
    root: Entity,
    (manifest, bus_path, bus_dir): (&BusManifest, &Path, &Path),
    (session_id, activate, pose): (crate::session_vehicles::VehicleId, bool, (Vec3, f32)),
) -> Vec3 {
    insert_vehicle_components(
        commands,
        root,
        (manifest, bus_path),
        (session_id, activate),
        pose,
        job.formation_cars.is_empty(),
    );
    let seed = script_seed(job.install.root(), bus_path, session_id);
    attach_vehicle_parts(job, commands, root, (manifest, bus_dir), (seed, activate));
    attach_trailers(
        job,
        commands,
        (&mut *meshes, &mut *materials, &mut *images),
        root,
        (bus_dir, manifest),
    );
    let cockpit_offset = cockpit_offset_of(manifest);
    seat_driver(
        (job, commands),
        (meshes, materials, images, bindposes),
        root,
        (bus_dir, manifest, cockpit_offset),
    );
    cockpit_offset
}
