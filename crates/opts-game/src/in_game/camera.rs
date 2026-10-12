//! Camera lifecycle management: toggling 2D menu camera vs 3D simulation camera.

use bevy::prelude::*;

/// Marker component for the persistent 2D UI / Menu Camera.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MenuCamera;

/// Deactivates the 2D menu camera once a 3D scene camera exists. Until then the menu camera is
/// the only thing that draws the loading screen (and its *Failed* outcome, which never gets a
/// scene camera); switching it off on entering the session left the window black while loading.
pub fn deactivate_menu_camera(
    scene: Query<&Camera, (With<Camera3d>, Without<MenuCamera>)>,
    mut query: Query<&mut Camera, With<MenuCamera>>,
) {
    if !scene.iter().any(|c| c.is_active) {
        return;
    }
    for mut camera in &mut query {
        if camera.is_active {
            camera.is_active = false;
            tracing::debug!("Deactivated 2D MenuCamera for 3D simulation session");
        }
    }
}

/// Reactivates the 2D menu camera when exiting in-game simulation back to menus.
pub fn reactivate_menu_camera(mut query: Query<&mut Camera, With<MenuCamera>>) {
    for mut camera in &mut query {
        camera.is_active = true;
        tracing::debug!("Reactivated 2D MenuCamera for main menu navigation");
    }
}
