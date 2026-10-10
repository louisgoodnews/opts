//! Toggleable diagnostic overlay.

use bevy::prelude::*;

use crate::in_game::types::InGameEntity;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::WorldStats;

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
            "[F10] map: {}\ntile: {:?} | tiles loaded: {} (missing {})\nscenery objects: {} (trees {}, protected-skipped {}, failed {}, no mesh {}) | splines: {} (failed {})\nvehicle: {} ({} meshes, {} verts)\nscene vertices: {} | asset failures: {}\nscrambled meshes: decoded {}, plain twins {}, decoding {}",
            stats.map_name,
            stats.start_tile,
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
