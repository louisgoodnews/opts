//! The environment, the plugins of a drive, and what runs when it starts and ends.

use super::*;

/// The environment, the plugins of a drive and what runs when it starts and ends.
pub(super) fn add_lifecycle(app: &mut App) {
    if !app.is_plugin_added::<opts_bevy::EnvironmentPlugin>() {
        app.add_plugins(opts_bevy::EnvironmentPlugin);
    }
    // The session clock runs (and changes apply) only in a drive.
    app.configure_sets(
        Update,
        opts_bevy::EnvironmentSet.run_if(in_state(AppState::InGame)),
    );
    app.add_plugins(crate::session_vehicles::SessionVehiclesPlugin);
    app.add_plugins((
        moon::MoonPlugin,
        opts_bevy::sun_rays::SunRaysPlugin,
        opts_bevy::reflections::ReflectionsPlugin,
    ));
    app.add_message::<crate::pause::PauseRequest>();
    app.add_plugins((VehicleMenuPlugin, FormationMenuPlugin))
        .add_systems(
            Update,
            omsi_world::handle_vehicle_requests.run_if(in_state(AppState::InGame)),
        );

    // Lifecycle hooks for entering/exiting AppState::InGame
    app.add_systems(
        FixedUpdate,
        omsi_world::rail_player_step
            .in_set(opts_bevy::RailSet)
            .after(opts_bevy::vehicle_physics_step_system)
            .run_if(in_state(AppState::InGame)),
    );
    app.add_systems(
        FixedUpdate,
        omsi_world::player_collisions
            .after(opts_bevy::vehicle_physics_step_system)
            .after(opts_bevy::RailSet)
            .before(opts_bevy::articulation_system)
            .run_if(in_state(AppState::InGame)),
    );
    app.add_systems(
        FixedUpdate,
        omsi_world::sample_consist_ground
            .after(opts_bevy::vehicle_physics_step_system)
            .after(opts_bevy::RailSet)
            .after(omsi_world::player_collisions)
            .before(opts_bevy::articulation_system)
            .run_if(in_state(AppState::InGame)),
    );
    app.add_systems(
        Update,
        omsi_world::attach_traffic_colliders.run_if(in_state(AppState::InGame)),
    );
    app.add_systems(
        OnEnter(AppState::InGame),
        (
            setup_in_game_session,
            omsi_world::configure_pedestrians,
            omsi_world::configure_rail,
            |mut i: ResMut<omsi_world::CollisionIndex>| *i = omsi_world::CollisionIndex::default(),
            omsi_world::configure_traffic,
        ),
    )
    .add_systems(
        OnExit(AppState::InGame),
        (
            omsi_world::log_final_position,
            |mut t: ResMut<Time<Virtual>>| t.unpause(),
            cleanup_in_game_session,
            reactivate_menu_camera,
        )
            .chain(),
    );

    loading_ui::build(app);
}
