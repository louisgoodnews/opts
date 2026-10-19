//! The doors of the player's vehicle for the door camera (M-0109).

use super::*;

/// Reads the passenger cabin and the paths file of the vehicle and puts the door points on its
/// root (`opts_bevy::door_camera::DoorPoints`); a vehicle without cabin or paths gets none.
pub(super) fn attach_door_points(
    job: &WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    manifest: &BusManifest,
    bus_dir: &Path,
) {
    let read = |rel: Option<&str>| {
        rel.and_then(|r| job.install.find_in(bus_dir, r))
            .and_then(|p| std::fs::read(p).ok())
    };
    let (Some(cabin), Some(paths)) = (
        read(manifest.passengercabin_cfg.as_deref()),
        read(manifest.paths_cfg.as_deref()),
    ) else {
        return;
    };
    let doors = opts_bevy::door_camera::door_points_from_bytes(&cabin, &paths);
    if !doors.is_empty() {
        tracing::info!(doors = doors.len(), "door camera points");
        commands
            .entity(root)
            .insert(opts_bevy::door_camera::DoorPoints(doors));
    }
}
