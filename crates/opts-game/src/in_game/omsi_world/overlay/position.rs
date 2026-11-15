//! Position tracking: the camera and vehicle tile in [`WorldStats`], and their log lines.

use bevy::prelude::*;
use opts_core::formats::map::TileGrid;

use super::super::{PositionReadout, WorldPos, WorldStats};

/// Formats one position as the panel's `label tile .. local .. | world ..` line.
pub(super) fn format_pos(label: &str, p: Option<WorldPos>, grid: &TileGrid) -> String {
    match p {
        Some(p) => {
            // Absolute coordinates are millions of metres: f32 keeps only half a metre, so the
            // panel computes them again from the tile and the local place in f64.
            let abs = grid.absolute(p.tile, p.local.as_dvec2());
            format!(
                "{label} tile {:?} local ({:.1}, {:.1}) | world ({:.1}, {:.1}, {:.1})",
                p.tile, p.local.x, p.local.y, abs.x, abs.y, p.world.z
            )
        }
        None => format!("{label} N/A"),
    }
}

/// Converts a Bevy-space position (x east, y up, z = −north, relative to the start tile's corner)
/// into an OMSI-frame position.
fn world_pos(grid: &TileGrid, start_tile: (i32, i32), p: Vec3) -> WorldPos {
    let (tile, local) = grid.tile_at(Vec2::new(p.x, -p.z).as_dvec2(), start_tile);
    let abs = grid.absolute(tile, local);
    WorldPos {
        tile,
        world: Vec3::new(abs.x as f32, abs.y as f32, p.y),
        local: local.as_vec2(),
    }
}

/// Tiles the camera and the vehicle were last logged in.
type LastTiles = (Option<(i32, i32)>, Option<(i32, i32)>);

/// Keeps [`WorldStats::position`] current and logs where the player is: `DEBUG` whenever the
/// camera or vehicle enters another tile, `TRACE` every 5 seconds, `INFO` when the session ends.
pub fn track_position(
    time: Res<Time>,
    mut since_trace: Local<f32>,
    mut last: Local<LastTiles>,
    mut stats: ResMut<WorldStats>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    vehicle: Query<&GlobalTransform, With<opts_bevy::PlayerVehicle>>,
    mut ended: Local<Option<PositionReadout>>,
) {
    let (start, grid) = (stats.start_tile, stats.grid);
    let cam = camera
        .single()
        .ok()
        .map(|c| world_pos(&grid, start, c.translation()));
    let veh = vehicle.single().ok().map(|v| {
        let (yaw, _, _) = v.rotation().to_euler(EulerRot::YXZ);
        // Bevy yaw is counter-clockwise about +Y; OMSI headings are clockwise from north.
        (
            world_pos(&grid, start, v.translation()),
            (-yaw.to_degrees()).rem_euclid(360.0),
        )
    });
    let readout = PositionReadout {
        camera: cam,
        vehicle: veh,
    };
    if stats.position != readout {
        stats.position = readout;
    }
    *ended = Some(readout);
    opts_bevy::set_position_tag(veh.map(|(v, _)| v).or(cam).map_or_else(String::new, |p| {
        format!(
            "tile {:?} local ({:.0}, {:.0}) world ({:.0}, {:.0}, {:.0})",
            p.tile, p.local.x, p.local.y, p.world.x, p.world.y, p.world.z
        )
    }));
    if let Some(c) = cam {
        if last.0 != Some(c.tile) {
            tracing::debug!(tile = ?c.tile, local = ?c.local, world = ?c.world, "camera entered tile");
            last.0 = Some(c.tile);
        }
    }
    if let Some((v, h)) = veh {
        if last.1 != Some(v.tile) {
            tracing::debug!(tile = ?v.tile, local = ?v.local, world = ?v.world, heading = h, "vehicle entered tile");
            last.1 = Some(v.tile);
        }
    }
    *since_trace += time.delta_secs();
    if *since_trace >= 5.0 {
        *since_trace = 0.0;
        tracing::trace!(camera = ?cam, vehicle = ?veh, "position");
    }
}

/// Logs the last known position when the session is left.
pub fn log_final_position(stats: Res<WorldStats>) {
    tracing::info!(
        camera = ?stats.position.camera,
        vehicle = ?stats.position.vehicle,
        "session ended at"
    );
}
