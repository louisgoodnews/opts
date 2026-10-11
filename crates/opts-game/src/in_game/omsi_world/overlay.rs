//! Toggleable diagnostic overlay.

use bevy::prelude::*;

use crate::in_game::types::InGameEntity;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{PositionReadout, WorldPos, WorldStats};
use opts_bevy::TILE_SIZE_M;

/// Marker for the toggleable diagnostic overlay text.
#[derive(Component)]
pub struct DebugOverlayText;

/// Spawns the (initially visible) diagnostic overlay.
pub fn spawn_debug_overlay(commands: &mut Commands) {
    commands.spawn((
        (
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 1.0, 0.85)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(70.0),
                left: Val::Px(12.0),
                ..default()
            },
        ),
        DebugOverlayText,
        InGameEntity,
    ));
}

/// Refreshes the overlay text; F3 toggles visibility.
pub fn update_debug_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<WorldStats>,
    mut query: Query<(&mut Text, &mut Visibility), With<DebugOverlayText>>,
) {
    if keyboard.just_pressed(KeyCode::F10) {
        stats.overlay_visible = !stats.overlay_visible;
    }
    for (mut text, mut vis) in &mut query {
        *vis = if stats.overlay_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        text.0 = format!(
            "[F10] map: {} (start tile {:?})\n{}\n{}\ntiles loaded: {} (missing {})\nscenery objects: {} (trees {}, protected-skipped {}, failed {}, no mesh {}) | splines: {} (failed {})\nvehicle: {} ({} meshes, {} verts)\nscene vertices: {} | asset failures: {}\nscrambled meshes: decoded {}, plain twins {}, decoding {}",
            stats.map_name,
            stats.start_tile,
            format_pos("camera ", stats.position.camera),
            stats.position.vehicle.map_or_else(
                || "vehicle: N/A".to_string(),
                |(p, h)| format!("{} heading {h:.0}°", format_pos("vehicle", Some(p))),
            ),
            stats.tiles_loaded,
            stats.tiles_missing,
            stats.objects_loaded,
            stats.trees_loaded,
            stats.objects_protected,
            stats.objects_failed,
            stats.objects_no_geometry,
            stats.splines_loaded,
            stats.splines_failed,
            if stats.vehicle_name.is_empty() { "N/A" } else { &stats.vehicle_name },
            stats.vehicle_meshes,
            stats.vehicle_vertices,
            stats.scene_vertices,
            stats.asset_failures,
            stats.meshes_decoded,
            stats.meshes_twin,
            stats.meshes_decoding,
        );
    }
}

fn format_pos(label: &str, p: Option<WorldPos>) -> String {
    match p {
        Some(p) => format!(
            "{label} tile {:?} local ({:.1}, {:.1}) | world ({:.1}, {:.1}, {:.1})",
            p.tile, p.local.x, p.local.y, p.world.x, p.world.y, p.world.z
        ),
        None => format!("{label} N/A"),
    }
}

/// Converts a Bevy-space position (x east, y up, z = −north, relative to the start tile's corner)
/// into an OMSI-frame position.
fn world_pos(start_tile: (i32, i32), p: Vec3) -> WorldPos {
    let (ox, oy) = (
        start_tile.0 as f32 * TILE_SIZE_M,
        start_tile.1 as f32 * TILE_SIZE_M,
    );
    let (x, y) = (ox + p.x, oy - p.z);
    let tile = (
        (x / TILE_SIZE_M).floor() as i32,
        (y / TILE_SIZE_M).floor() as i32,
    );
    WorldPos {
        tile,
        world: Vec3::new(x, y, p.y),
        local: Vec2::new(
            x - tile.0 as f32 * TILE_SIZE_M,
            y - tile.1 as f32 * TILE_SIZE_M,
        ),
    }
}

/// Keeps [`WorldStats::position`] current and logs where the player is: `DEBUG` whenever the
/// camera or vehicle enters another tile, `TRACE` every 5 seconds, `INFO` when the session ends.
pub fn track_position(
    time: Res<Time>,
    mut since_trace: Local<f32>,
    mut last: Local<(Option<(i32, i32)>, Option<(i32, i32)>)>,
    mut stats: ResMut<WorldStats>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    vehicle: Query<&GlobalTransform, With<opts_bevy::PlayerVehicle>>,
    mut ended: Local<Option<PositionReadout>>,
) {
    let start = stats.start_tile;
    let cam = camera
        .single()
        .ok()
        .map(|c| world_pos(start, c.translation()));
    let veh = vehicle.single().ok().map(|v| {
        let (yaw, _, _) = v.rotation().to_euler(EulerRot::YXZ);
        // Bevy yaw is counter-clockwise about +Y; OMSI headings are clockwise from north.
        (
            world_pos(start, v.translation()),
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
