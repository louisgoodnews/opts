//! The driver: the cockpit eye point of the standard camera and the seating of the character.

use super::*;

/// The driver's eye point of the standard camera, in Bevy space (vehicle space → Bevy by the
/// shared helper).
pub(super) fn cockpit_offset_of(manifest: &BusManifest) -> Vec3 {
    let driver_cam = manifest
        .driver_cameras
        .get(manifest.std_camera_index.unwrap_or(0))
        .or_else(|| manifest.driver_cameras.first());
    driver_cam
        .map(|c| {
            camera_preset_to_bevy_transform(
                Vec3::new(c.x, c.y, c.z),
                c.dist,
                c.yaw_deg,
                c.pitch_deg,
            )
            .translation
        })
        .unwrap_or(Vec3::new(0.0, 1.2, 0.0))
}

/// Seats the player's character at the driver position.
pub(super) fn seat_driver(
    (job, commands): (&mut WorldLoadJob, &mut Commands),
    (meshes, materials, images, bindposes): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
        &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    ),
    root: Entity,
    (bus_dir, manifest, cockpit_offset): (&Path, &BusManifest, Vec3),
) {
    match load_driver_human(
        (job, commands),
        (meshes, materials, images, bindposes),
        root,
        (
            bus_dir,
            manifest.passengercabin_cfg.as_deref(),
            cockpit_offset,
        ),
    ) {
        Ok(name) => tracing::info!(human = %name, "player character seated at the driver position"),
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), "cannot load the player character: {e}")
        }
    }
}
