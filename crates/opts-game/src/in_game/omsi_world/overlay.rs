//! Toggleable diagnostic overlay.

use bevy::prelude::*;

use crate::in_game::types::InGameEntity;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{PositionReadout, WorldPos, WorldStats};

/// Marker for the toggleable diagnostic overlay text.
#[derive(Component)]
pub struct DebugOverlayText;

/// Spawns the developer panel (hidden unless *Settings → Advanced → Developer tools* is on). It
/// uses the instrument tokens like the rest of the HUD.
pub fn spawn_debug_overlay(commands: &mut Commands) {
    commands.spawn((
        (
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(96.0),
                left: Val::Px(20.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Visibility::Hidden,
        ),
        crate::in_game::instruments::InstrumentPanel,
        crate::in_game::instruments::InstrumentInk::Text,
        DebugOverlayText,
        InGameEntity,
    ));
}

/// F10 switches the developer panel by changing its setting (so the page and the key agree and
/// the choice is remembered).
pub fn toggle_developer_panel(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut api: crate::settings_registry::SettingsApi,
) {
    if !keyboard.just_pressed(KeyCode::F10) {
        return;
    }
    let on = api.text("advanced.developer_panel") == "true";
    let next = if on { "false" } else { "true" };
    if let Err(e) = api.change(
        "advanced.developer_panel",
        Some(next),
        crate::settings_registry::ChangeSource::Page,
        true,
    ) {
        tracing::warn!("developer panel setting: {e}");
    }
}

/// Refreshes the developer panel text while it is visible.
pub fn update_debug_overlay(
    stats: Res<WorldStats>,
    mut query: Query<(&mut Text, &mut Visibility), With<DebugOverlayText>>,
) {
    for (mut text, mut vis) in &mut query {
        *vis = if stats.overlay_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !stats.overlay_visible {
            continue;
        }
        text.0 = format!(
            "[F10] map: {} (start tile {:?})\n{}\n{}\ntiles loaded: {} (missing {})\nscenery objects: {} (trees {}, protected-skipped {}, failed {}, no mesh {}) | splines: {} (failed {})\nvehicle: {} ({} meshes, {} verts)\nscene vertices: {} | asset failures: {} | load problems: {}\nscrambled meshes: decoded {}, plain twins {}, decoding {}",
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
            stats.load_problems,
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
        start_tile.0 as f32 * opts_bevy::tile_size(),
        start_tile.1 as f32 * opts_bevy::tile_size(),
    );
    let (x, y) = (ox + p.x, oy - p.z);
    let tile = (
        (x / opts_bevy::tile_size()).floor() as i32,
        (y / opts_bevy::tile_size()).floor() as i32,
    );
    WorldPos {
        tile,
        world: Vec3::new(x, y, p.y),
        local: Vec2::new(
            x - tile.0 as f32 * opts_bevy::tile_size(),
            y - tile.1 as f32 * opts_bevy::tile_size(),
        ),
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
