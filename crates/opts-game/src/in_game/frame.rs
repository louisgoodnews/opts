//! The systems that run every frame of a drive.

use super::*;

/// The systems that run every frame of a drive.
/// Systems that follow the streamed world: the Chrono map-date switch and the zone entities.
fn add_world_systems(app: &mut App) {
    // Chrono: the live switch on a new map date, and its progress for the pause page.
    app.add_systems(
        Update,
        (
            omsi_world::apply_map_date_changes
                .after(opts_bevy::EnvironmentSet)
                .before(omsi_world::stream_world_tiles),
            omsi_world::track_chrono_rebuild.after(omsi_world::stream_world_tiles),
        )
            .run_if(in_state(AppState::InGame)),
    );

    // M-0115: zone entities follow the resident tiles; runs without a world too, to drop them.
    app.add_systems(
        Update,
        omsi_world::sync_tile_zones
            .after(omsi_world::stream_world_tiles)
            .run_if(resource_exists::<opts_bevy::spatial_zones::ZoneRegistry>),
    );
}

pub(super) fn add_frame_systems(app: &mut App) {
    // HUD instrument colours: day/night palette by the sky's time of day
    app.add_systems(
        Update,
        (
            instruments::update_instrument_variant,
            instruments::sync_instrument_tokens,
        )
            .chain()
            .after(opts_bevy::EnvironmentSet)
            .run_if(in_state(AppState::InGame)),
    );

    add_world_systems(app);

    #[cfg(feature = "stream-trace")]
    app.add_systems(Last, stream_trace_frame);

    // Active simulation frame updates
    app.add_systems(
        Update,
        (
            update_loading_progress,
            deactivate_menu_camera,
            omsi_world::stream_world_tiles,
            omsi_world::track_position,
            omsi_world::warn_vehicle_in_water,
            toggle_pause,
            debug_frame_log
                .after(omsi_world::follow_ground)
                .after(omsi_world::rail_player_pose),
            omsi_world::load_rail_variants,
            omsi_world::spawn_ai_trains,
            omsi_world::move_ai_trains,
            omsi_world::rail_player_pose.after(opts_bevy::vehicle_transform_sync_system),
            omsi_world::follow_ground.after(opts_bevy::vehicle_transform_sync_system),
            (
                omsi_world::update_vehicle_text_textures,
                omsi_world::update_vehicle_script_textures,
                opts_bevy::text_texture::rebind_redrawn_images
                    .after(omsi_world::update_vehicle_text_textures)
                    .after(omsi_world::update_vehicle_script_textures),
            ),
            (
                omsi_world::load_variants,
                omsi_world::spawn_pedestrians,
                omsi_world::move_pedestrians,
            )
                .chain(),
            (
                omsi_world::update_traffic_graph,
                // The player's footprints first: spawning must not put a vehicle inside the
                // bus, and the step must see where it stands.
                omsi_world::publish_traffic_obstacles,
                omsi_world::load_traffic_variants,
                omsi_world::plan_scheduled,
                omsi_world::spawn_traffic,
                omsi_world::move_traffic,
                omsi_world::move_traffic_chains,
                omsi_world::update_ai_variables,
                omsi_world::step_ai_scripts.after(omsi_world::update_ai_variables),
            )
                .chain(),
            sky::update_sky
                .run_if(resource_exists::<sky::Sky>)
                .after(opts_bevy::EnvironmentSet),
            (world::update_fallback_light, sky::place_sky_dome).after(opts_bevy::EnvironmentSet),
            omsi_world::place_dev_camera,
            omsi_world::sync_scenery_clock
                .run_if(resource_exists::<opts_bevy::SceneryClock>)
                .after(opts_bevy::EnvironmentSet)
                .before(opts_bevy::scenery_runtime::SceneryRuntimeSet),
            screenshot::capture_screenshot_when_ready,
        )
            .run_if(in_state(AppState::InGame)),
    );
}
