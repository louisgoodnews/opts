//! In-game transit driving simulation lifecycle, loading orchestration, and telemetry HUD.

pub mod attachments;
pub mod camera;
pub mod change_map;
pub mod hot_reload;
pub mod hud;
pub mod instruments;
pub mod load_report;
pub mod loading_pipeline;
pub mod loading_ui;
pub mod moon;
pub mod omsi_world;
pub mod screenshot;
pub mod sky;
pub mod sky_cube;
pub mod stream_budget;
pub mod tile_loading;
pub mod types;
pub mod vehicle_spawner;
pub mod weather;
pub mod world;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::in_game::camera::{deactivate_menu_camera, reactivate_menu_camera};
use crate::in_game::hud::handle_hud_interactions;
use crate::in_game::loading_pipeline::{handle_loading_interactions, update_loading_progress};
use crate::in_game::loading_ui::spawn_loading_screen_ui;
use crate::new_game_screen::types::NewGameSetup;

mod formation_menu;
mod session;
mod vehicle_menu;

pub(crate) use session::setup_in_game_session;
#[cfg(feature = "stream-trace")]
use session::stream_trace_frame;
use session::{cleanup_in_game_session, debug_frame_log};

pub use camera::MenuCamera;
pub use formation_menu::{FormationCommands, FormationEditor, FormationMenuPlugin};
pub use types::{
    CancelLoadingBtn, InGameCamera, InGameEntity, InGameHudRoot, InGameSessionInfo, LoadStage,
    LoadingScreenRoot, PauseMenuBtn, ProgressBarFill, SimulationLoadState, SpeedometerText,
    TachometerText,
};
pub use vehicle_menu::{MenuCommands, VehicleMenu, VehicleMenuPlugin};

/// The resources of a drive.
fn add_resources(app: &mut App) {
    app.init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
        .init_resource::<omsi_world::GroundIndex>()
        .init_resource::<opts_bevy::text_texture::RedrawnImages>()
        .init_resource::<omsi_world::ChronoState>()
        .init_resource::<omsi_world::DeveloperDetail>()
        .init_resource::<omsi_world::CollisionIndex>()
        .init_resource::<omsi_world::RailNetwork>()
        .init_resource::<omsi_world::BikeNetwork>()
        .init_resource::<omsi_world::BikeSettings>()
        .init_resource::<omsi_world::BikeAiPool>()
        .init_resource::<omsi_world::RailSettings>()
        .init_resource::<omsi_world::RailAiPool>()
        .init_resource::<omsi_world::PedestrianNetwork>()
        .init_resource::<omsi_world::PedestrianSettings>()
        .init_resource::<omsi_world::PedestrianPool>()
        .init_resource::<omsi_world::TrafficState>()
        .init_resource::<omsi_world::TrafficSettings>()
        .init_resource::<omsi_world::TrafficPool>()
        .init_resource::<omsi_world::ScheduledTraffic>()
        .init_resource::<SimulationLoadState>()
        .init_resource::<InGameSessionInfo>()
        .init_resource::<opts_bevy::CameraState>()
        .init_resource::<opts_bevy::CameraAvailability>()
        .insert_resource(omsi_world::WorldStats {
            // The developer panel is off unless the setting says otherwise.
            overlay_visible: false,
            ..default()
        });
}

/// The environment, the plugins of a drive and what runs when it starts and ends.
fn add_lifecycle(app: &mut App) {
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

/// The systems that run every frame of a drive.
fn add_frame_systems(app: &mut App) {
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

/// Scheduled trains, rail scripts, the sky cube and the developer panel.
fn add_late_systems(app: &mut App) {
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

/// Bevy plugin managing the active transit simulation lifecycle, loading sequence, and driving HUD.
#[derive(Default)]
pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        add_resources(app);
        add_lifecycle(app);
        add_frame_systems(app);
        add_late_systems(app);
    }
}

/// `P` pauses and resumes the simulation (virtual time; the free camera and the HUD keep
/// running). Ignored while the F1 report modal is open, whose text field takes the key.
fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    menu: Option<Res<opts_bevy::reporting::MenuModalActive>>,
    report: Option<Res<State<opts_bevy::reporting::ReportModalState>>>,
    mut time: ResMut<Time<Virtual>>,
) {
    if report.is_some_and(|r| *r.get() != opts_bevy::reporting::ReportModalState::Inactive)
        || menu.is_some_and(|m| m.0)
        || !keyboard.just_pressed(KeyCode::KeyP)
    {
        return;
    }
    if time.is_paused() {
        time.unpause();
        tracing::info!("simulation resumed [P]");
    } else {
        time.pause();
        tracing::info!("simulation paused [P]");
    }
}
