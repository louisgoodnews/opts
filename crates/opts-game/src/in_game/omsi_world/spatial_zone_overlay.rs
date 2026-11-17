//! The spatial-zone debug overlay (M-0115): the `spatial_zone_overlay` action (Shift+F12) switches
//! the zone boxes and the subject markers. Off by default; the drawing is in
//! `opts_bevy::spatial_zones::overlay`.

use bevy::prelude::*;
use opts_bevy::spatial_zones::overlay::{draw_zone_overlay, ZoneOverlay};

use crate::app_state::AppState;

/// Shift+F12 (the `spatial_zone_overlay` action) switches the overlay.
pub fn toggle_zone_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut overlay: ResMut<ZoneOverlay>,
) {
    if opts_bevy::action_keys_of(&action_keys).just_pressed(&keyboard, "spatial_zone_overlay") {
        overlay.toggle();
    }
}

/// Registers the toggle and the drawing (called from the in-game late systems).
pub(crate) fn add_spatial_zone_overlay(app: &mut App) {
    app.add_systems(
        Update,
        toggle_zone_overlay
            .run_if(in_state(AppState::InGame))
            .run_if(opts_bevy::reporting::not_modal_active)
            .run_if(resource_exists::<ZoneOverlay>),
    )
    .add_systems(
        Update,
        draw_zone_overlay
            .run_if(in_state(AppState::InGame))
            .run_if(opts_bevy::reporting::not_modal_active)
            .run_if(resource_exists::<ZoneOverlay>)
            .run_if(resource_exists::<bevy::gizmos::config::GizmoConfigStore>),
    );
}
