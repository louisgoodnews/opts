//! The late systems of a drive: scheduled trains, rail scripts, the sky cube and the developer panel.

use super::*;

/// Scheduled trains, rail scripts, the sky cube and the developer panel.
pub(super) fn add_late_systems(app: &mut App) {
    app.add_systems(
        Update,
        omsi_world::spawn_scheduled_trains
            .after(omsi_world::spawn_ai_trains)
            .before(omsi_world::move_ai_trains)
            .run_if(in_state(AppState::InGame)),
    );
    // Also added by the vehicle trigger plugin; adding it here keeps headless apps without it valid.
    app.add_message::<opts_bevy::SceneryClicked>();
    app.add_systems(
        Update,
        (
            omsi_world::step_rail_scripts.after(omsi_world::move_ai_trains),
            omsi_world::switch_hint_system,
            omsi_world::set_points_system.before(omsi_world::move_ai_trains),
            omsi_world::points_visual_system,
            omsi_world::click_points_system.before(omsi_world::set_points_system),
        )
            .run_if(in_state(AppState::InGame)),
    );

    // The sky cube map for glass reflections reads the sky that `sky::update_sky` keeps current.
    app.add_systems(
        Update,
        sky_cube::feed_sky_cube
            .after(sky::update_sky)
            .after(opts_bevy::EnvironmentSet)
            .run_if(in_state(AppState::InGame)),
    );

    // F10 changes the developer panel setting (needs the settings registry).
    app.add_systems(
        Update,
        (
            omsi_world::toggle_developer_panel,
            omsi_world::toggle_developer_detail,
        )
            .run_if(in_state(AppState::InGame))
            .run_if(opts_bevy::reporting::not_modal_active)
            .run_if(resource_exists::<crate::settings_registry::SettingsRegistry>),
    );

    omsi_world::traffic::lane_overlay::add_lane_overlay(app);
    // Input-driven systems stand still while the report modal is open.
    app.add_systems(
        Update,
        (
            handle_loading_interactions,
            omsi_world::update_debug_overlay,
            omsi_world::update_wheel_vars_overlay,
            handle_hud_interactions,
        )
            .run_if(in_state(AppState::InGame))
            .run_if(opts_bevy::reporting::not_modal_active),
    );
}
